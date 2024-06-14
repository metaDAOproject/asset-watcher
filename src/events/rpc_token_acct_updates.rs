use std::sync::Arc;

use diesel::PgConnection;
use futures::StreamExt;
use solana_account_decoder::{UiAccount, UiAccountData};
use solana_client::{nonblocking::pubsub_client::PubsubClient, rpc_config::RpcAccountInfoConfig};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};

use crate::entities::token_accts::TokenAcct;
use crate::services::balances;

pub async fn new_handler(
    pub_sub_client: Arc<PubsubClient>,
    db: &mut PgConnection,
    token_acct_pubkey: Pubkey,
    token_acct_record: TokenAcct,
) {
    println!("subscribing to account {}", token_acct_pubkey.to_string());
    let (mut subscription, unsubscribe) = pub_sub_client
        .account_subscribe(
            &token_acct_pubkey,
            Some(RpcAccountInfoConfig {
                encoding: Some(solana_account_decoder::UiAccountEncoding::JsonParsed),
                data_slice: None,
                commitment: Some(CommitmentConfig::confirmed()),
                min_context_slot: None,
            }),
        )
        .await
        .expect("Failed to subscribe to account");

    while let Some(val) = subscription.next().await {
        let ui_account: UiAccount = val.value;
        let context = val.context;
        println!("account subscribe context: {:?}", context);
        match ui_account.data {
            UiAccountData::Binary(data, encoding) => {
                println!("Binary data: {:?}, Encoding: {:?}", data, encoding);
                // Process binary data here
            }
            UiAccountData::Json(data) => {
                println!("account subscribe notification: {:?}", data);
                let record_clone = token_acct_record.clone();
                let token_acct_clone = record_clone.token_acct.clone();
                let token_acct_update_res =
                    balances::handle_token_acct_change(db, record_clone, data, context);
                match token_acct_update_res {
                    Ok(_) => {
                        println!("successfully updated token balance: {:?}", token_acct_clone)
                    }
                    Err(e) => println!("error kind: {:?}", e),
                }
                // Process JSON data here
            }
            UiAccountData::LegacyBinary(data) => {
                println!("Parsed LegacyBinary data: {:?}", data);
                // Process parsed JSON data here
            }
        }
    }
    unsubscribe();
    println!(
        "end of rpc account subscriber scope, unsubscribing from account: {}",
        token_acct_pubkey.to_string()
    );
}
