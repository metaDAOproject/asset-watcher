use crate::entities::token_accts::token_accts;
use crate::entities::token_accts::TokenAcct;
use crate::entities::token_accts::TokenAcctsInsertChannelPayload;
use crate::events::rpc_token_acct_updates;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::PgConnection;
use futures::{stream, FutureExt, TryStreamExt};
use futures_util::StreamExt;
use postgres::NoTls;
use postgres::Notification;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tokio_postgres::{connect, AsyncMessage};

use crate::entities::token_accts::token_accts::dsl::*;

pub async fn new_handler(
    db_url: &str,
    pool_connection: Arc<Object<Manager<PgConnection>>>,
    pub_sub_rpc_client: Arc<PubsubClient>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (client, mut connection) = connect(db_url, NoTls).await.unwrap();
    // Make transmitter and receiver.
    let (tx, mut rx) = futures_channel::mpsc::unbounded();
    let stream =
        stream::poll_fn(move |cx| connection.poll_message(cx)).map_err(|e| panic!("{}", e));
    let connection = stream.forward(tx).map(|r| r.unwrap());
    tokio::spawn(connection);

    client
        .batch_execute("LISTEN token_accts_insert_channel;")
        .await
        .unwrap();

    while let Some(m) = rx.next().await {
        let connect_clone = Arc::clone(&pool_connection);
        match m {
            AsyncMessage::Notification(n) => match n.channel() {
                "token_accts_insert_channel" => {
                    handle_new_token_acct_notification(
                        connect_clone,
                        n,
                        Arc::clone(&pub_sub_rpc_client),
                    )
                    .await?
                }
                _ => (),
            },
            AsyncMessage::Notice(notice) => println!("async message error: {:?}", notice),
            _ => println!("fallthrough handler of async message from postgres listener"),
        }
    }
    Ok(())
}

async fn handle_new_token_acct_notification(
    pool_connection: Arc<Object<Manager<PgConnection>>>,
    notification: Notification,
    pub_sub_rpc_client: Arc<PubsubClient>,
) -> Result<(), Box<dyn std::error::Error>> {
    let cloned_connection = Arc::clone(&pool_connection);
    let token_acct_payload = TokenAcctsInsertChannelPayload::parse_payload(notification.payload())?;
    let token_acct_string = token_acct_payload.token_acct;
    let acct = token_acct_string.clone();
    let token_acct_record: TokenAcct = cloned_connection
        .clone()
        .interact(move |conn| {
            return token_accts
                .filter(token_accts::dsl::token_acct.eq(&acct))
                .first(conn)
                .expect("could not find token record");
        })
        .await?;
    let token_acct_pubkey = Pubkey::from_str(&token_acct_string)?;
    let conn_manager_arg_clone = Arc::clone(&pool_connection);
    let pub_sub_client_clone = Arc::clone(&pub_sub_rpc_client);

    tokio::spawn(async move {
        rpc_token_acct_updates::new_handler(
            pub_sub_client_clone,
            conn_manager_arg_clone,
            token_acct_pubkey,
            token_acct_record.clone(),
        )
        .await
    });

    Ok(())
}
