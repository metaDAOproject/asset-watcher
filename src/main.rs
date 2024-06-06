#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::prelude::*;
use dotenv::dotenv;
use services::balances_handler::handle_token_acct_change;
use solana_account_decoder::{UiAccount, UiAccountData};
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use std::{env, sync::Arc};
use tokio::signal;
mod entities;
use entities::token_accts::{token_accts::dsl::*, TokenAcct};
mod adapters;
mod services;
use deadpool::managed::Object;
use deadpool_diesel::postgres::{Pool, Runtime};
use deadpool_diesel::Manager;
use futures_util::StreamExt;
use std::str::FromStr;

async fn get_database_pool(
) -> Result<Arc<Object<Manager<PgConnection>>>, Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = Manager::new(database_url, Runtime::Tokio1);
    let pool = Pool::builder(manager).max_size(8).build()?;
    let conn_manager = pool.get().await?;
    Ok(Arc::new(conn_manager))
}

async fn get_pubsub_client(
) -> Result<Arc<solana_client::nonblocking::pubsub_client::PubsubClient>, Box<dyn std::error::Error>>
{
    let rpc_endpoint_ws = env::var("RPC_ENDPOINT_WSS").expect("RPC_ENDPOINT_WSS must be set");
    let pub_sub_client =
        solana_client::nonblocking::pubsub_client::PubsubClient::new(&rpc_endpoint_ws).await?;
    Ok(Arc::new(pub_sub_client))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let conn_manager_arc = get_database_pool().await?;

    let results = conn_manager_arc
        .clone()
        .interact(|conn| {
            // TODO: add a status on token_accts to show which ones are still being indexed and which ones are not
            return token_accts
                .load::<TokenAcct>(conn)
                .expect("Error loading token_accts");
        })
        .await?;

    // let mut rpc_subs: Vec<PubsubClientSubscription<Response<UiAccount>>> = vec![];
    let pub_sub_client = get_pubsub_client().await?;
    for record in results {
        match Pubkey::from_str(&record.token_acct) {
            Ok(token_acct_pubkey) => {
                let conn_manager_arg_clone = Arc::clone(&conn_manager_arc);
                let pub_sub_client_clone = Arc::clone(&pub_sub_client);
                tokio::spawn(async move {
                    println!("subscribing to account {}", token_acct_pubkey.to_string());
                    let (mut subscription, _) = pub_sub_client_clone
                        .account_subscribe(
                            &token_acct_pubkey,
                            Some(RpcAccountInfoConfig {
                                encoding: Some(
                                    solana_account_decoder::UiAccountEncoding::JsonParsed,
                                ),
                                data_slice: None,
                                commitment: Some(CommitmentConfig::confirmed()),
                                min_context_slot: None,
                            }),
                        )
                        .await
                        .expect("Failed to subscribe to account");

                    while let Some(val) = subscription.next().await {
                        let ui_account: UiAccount = val.value;
                        match ui_account.data {
                            UiAccountData::Binary(data, encoding) => {
                                println!("Binary data: {:?}, Encoding: {:?}", data, encoding);
                                // Process binary data here
                            }
                            UiAccountData::Json(data) => {
                                let record_clone = record.clone();
                                let token_acct_clone = record_clone.token_acct.clone();
                                let token_acct_update_res = conn_manager_arg_clone
                                    .interact(move |conn| {
                                        return handle_token_acct_change(conn, record_clone, data);
                                    })
                                    .await;
                                match token_acct_update_res {
                                    Ok(res) => match res {
                                        Ok(_) => println!(
                                            "successfully updated token balance: {:?}",
                                            token_acct_clone
                                        ),
                                        Err(e) => println!("error kind: {:?}", e),
                                    },
                                    Err(e) => println!("interact error: {:?}", e),
                                }
                                // Process JSON data here
                            }
                            UiAccountData::LegacyBinary(data) => {
                                println!("Parsed LegacyBinary data: {:?}", data);
                                // Process parsed JSON data here
                            }
                        }
                    }
                });
            }
            Err(e) => eprintln!("Error with token acct pubkey parsing: {}", e),
        }
    }

    // Block the main function and handle CTRL+C
    signal::ctrl_c().await?;
    println!("Received CTRL+C, shutting down.");
    Ok(())
}
