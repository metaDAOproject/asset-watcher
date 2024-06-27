use crate::entities::token_accts::{token_accts, TokenAcct, TokenAcctStatus};
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::PgConnection;
use futures::{stream, FutureExt, StreamExt, TryStreamExt};
use postgres::NoTls;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tokio::task;
use tokio_postgres::{connect, AsyncMessage};

// TODO this should return a result
pub async fn setup_event_listeners(
    db_url: &str,
    managed_connection: Arc<Object<Manager<PgConnection>>>,
    pub_sub_client: Arc<PubsubClient>,
) {
    // account subscribe for token_accts already in Watching status
    let token_accts_res = managed_connection
        .clone()
        .interact(|conn| {
            return token_accts::table
                .filter(token_accts::status.eq(TokenAcctStatus::Watching))
                .load::<TokenAcct>(conn)
                .expect("Error loading token_accts");
        })
        .await;

    match token_accts_res {
        Ok(token_accts_vec) => {
            for record in token_accts_vec {
                match Pubkey::from_str(&record.token_acct) {
                    Ok(token_acct_pubkey) => {
                        let conn_manager_arg_clone = Arc::clone(&managed_connection);
                        let pub_sub_client_clone = Arc::clone(&pub_sub_client);
                        println!(
                            "spawning task for token acct subscription: {}",
                            token_acct_pubkey.to_string()
                        );
                        task::spawn(async move {
                            println!(
                                "task running for token acct subscription: {}",
                                token_acct_pubkey.to_string()
                            );
                            super::rpc_token_acct_updates::new_handler(
                                pub_sub_client_clone,
                                conn_manager_arg_clone,
                                token_acct_pubkey,
                                record,
                            )
                            .await
                        });
                    }
                    Err(e) => eprintln!("Error with token acct pubkey parsing: {}", e),
                }
            }
        }
        Err(e) => eprintln!("Error with subscribing to token accts: {}", e),
    }

    // listen to postgres notifications
    let (client, mut connection) = connect(db_url, NoTls).await.unwrap();
    // Make transmitter and receiver.
    let (tx, mut rx) = futures_channel::mpsc::unbounded();
    let stream =
        stream::poll_fn(move |cx| connection.poll_message(cx)).map_err(|e| panic!("{}", e));
    let connection = stream.forward(tx).map(|r| r.unwrap());
    tokio::spawn(connection);

    client
        .batch_execute(
            "
            LISTEN token_accts_insert_channel;
            LISTEN token_accts_status_update_channel;
            LISTEN transactions_insert_channel;
            ",
        )
        .await
        .unwrap();

    while let Some(m) = rx.next().await {
        let connect_clone = Arc::clone(&managed_connection);
        match m {
            AsyncMessage::Notification(n) => match n.channel() {
                "token_accts_insert_channel" => {
                    task::spawn(super::token_accts_insert::new_handler(
                        n,
                        connect_clone,
                        Arc::clone(&pub_sub_client),
                    ));
                }
                "transactions_insert_channel" => {
                    task::spawn(super::transactions_insert::new_handler(n, connect_clone));
                }
                "token_accts_status_update_channel" => {
                    task::spawn(super::token_accts_status_update::new_handler(
                        n,
                        connect_clone,
                        Arc::clone(&pub_sub_client),
                    ));
                }
                _ => (),
            },
            AsyncMessage::Notice(notice) => println!("async message error: {:?}", notice),
            _ => println!("fallthrough handler of async message from postgres listener"),
        }
    }
}
