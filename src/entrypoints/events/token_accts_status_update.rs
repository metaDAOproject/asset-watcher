use crate::entities::token_accts::token_accts;
use crate::entities::token_accts::TokenAcct;
use crate::entities::token_accts::TokenAcctStatus;
use crate::entities::token_accts::TokenAcctsStatusUpdateChannelPayload;
use crate::entrypoints::events::rpc_token_acct_updates;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::PgConnection;
use futures::executor::block_on;
use postgres::Notification;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;

use crate::entities::token_accts::token_accts::dsl::*;

pub async fn new_handler(
    notification: Notification,
    pool_connection: Arc<Object<Manager<PgConnection>>>,
    pub_sub_rpc_client: Arc<PubsubClient>,
) {
    println!(
        "new token_accts_status_update payload: {:?}",
        notification.payload()
    );
    match handle_update_token_acct_status_notification(
        pool_connection,
        notification,
        Arc::clone(&pub_sub_rpc_client),
    )
    .await
    {
        Ok(()) => println!("successfully handled new token_acct notification"),
        Err(e) => eprintln!("error handling new token_acct notification: {:?}", e),
    };
}

async fn handle_update_token_acct_status_notification(
    pool_connection: Arc<Object<Manager<PgConnection>>>,
    notification: Notification,
    pub_sub_rpc_client: Arc<PubsubClient>,
) -> Result<(), Box<dyn std::error::Error>> {
    let cloned_connection = Arc::clone(&pool_connection);
    let token_acct_payload =
        TokenAcctsStatusUpdateChannelPayload::parse_payload(notification.payload())?;
    if token_acct_payload.status != TokenAcctStatus::Watching {
        return Ok(());
    }
    let token_acct_string = token_acct_payload.token_acct;
    let token_acct_clone = token_acct_string.clone();
    let token_acct_record: TokenAcct = cloned_connection
        .clone()
        .interact(move |conn| {
            return token_accts
                .filter(token_accts::dsl::token_acct.eq(&token_acct_clone))
                .first(conn)
                .expect("could not find token record");
        })
        .await?;
    let token_acct_pubkey = Pubkey::from_str(&token_acct_string)?;
    let pub_sub_client_clone = Arc::clone(&pub_sub_rpc_client);

    tokio::spawn(async move {
        let res = cloned_connection
            .interact(move |conn: &mut PgConnection| {
                block_on(rpc_token_acct_updates::new_handler(
                    pub_sub_client_clone,
                    conn,
                    token_acct_pubkey,
                    token_acct_record.clone(),
                ))
            })
            .await;

        match res {
            Ok(_) => (),
            Err(e) => println!(
                "DB error with creating rpc token acct update handler for acct [{}]: {:?}",
                token_acct_string, e
            ),
        }
    })
    .await?;

    Ok(())
}
