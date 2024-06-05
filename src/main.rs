#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::prelude::*;
use dotenv::dotenv;
use solana_account_decoder::{parse_token::UiTokenAmount, UiAccount, UiAccountEncoding};
use solana_client::nonblocking::{
    pubsub_client::PubsubClientSubscription, rpc_config::RpcAccountInfoConfig,
    rpc_response::Response,
};
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use std::env;
use tokio::signal;
mod entities;
use entities::token_accts::{token_accts::dsl::*, TokenAcct};
mod adapters;
mod services;
use deadpool_diesel::postgres::{Manager, Pool, Runtime};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = Manager::new(database_url, Runtime::Tokio1);
    let pool = Pool::builder(manager).max_size(8).build()?;
    let conn_manager = pool.get().await?;
    // let other_connection = &mut establish_connection();

    let rpc_endpoint_ws = env::var("RPC_ENDPOINT_WSS").expect("RPC_ENDPOINT_WSS must be set");
    // let rpc_client = SolanaRpcClient::new(&rpc_endpoint_ws).await?;
    // let rpc_client_arc = Arc::new(Mutex::new(rpc_client));
    // let arc_connection = Arc::new(*connection);
    // let balances_handler = BalancesHandler::new(arc_connection);

    let results = conn_manager
        .interact(|conn| {
            return token_accts
                .filter(owner_acct.eq("HwBL75xHHKcXSMNcctq3UqWaEJPDWVQz6NazZJNjWaQc"))
                .load::<TokenAcct>(conn)
                .expect("Error loading token_accts");
        })
        .await?;

    let mut rpc_subs: Vec<PubsubClientSubscription<Response<UiAccount>>> = vec![];
    for record in results {
        match Pubkey::from_str(&record.token_acct) {
            Ok(token_acct_pubkey) => {
                let (subscription, receiver) =
                    solana_client::pubsub_client::PubsubClient::account_subscribe(
                        &rpc_endpoint_ws,
                        &token_acct_pubkey,
                        Some(RpcAccountInfoConfig {
                            encoding: None,
                            data_slice: None,
                            commitment: Some(CommitmentConfig::confirmed()),
                            min_context_slot: None,
                        }),
                    )?;

                loop {
                    match subscription. {
                        Ok(response) => {
                            println!("account subscription response: {:?}", response);
                        }
                        Err(e) => {
                            println!("account subscription error: {:?}", e);
                            break;
                        }
                    }
                }
                // for val in subscription. {
                //     let ui_account: UiAccount = val.value;
                //     let account_data = ui_account.data.decode();
                //     // how to decode the solana token balance account in rusty here
                // }
                // tokio::spawn(async move {
                //     while let Some(val) = receiver. {
                //         let ui_account: UiAccount = val.value;
                //         if let Some(account_data) = ui_account.data {
                //             println!("account data: {:?}", account_data);
                //             // if let Some(balance) = decode_token_balance(&account_data) {
                //             //     println!("Token account balance: {:?}", balance);
                //             // }
                //         }
                //     }
                // });
                rpc_subs.push(subscription);
            }
            Err(e) => eprintln!("Error with token acct pubkey parsing: {}", e),
        }
    }

    // Block the main function and handle CTRL+C
    signal::ctrl_c().await?;
    println!("Received CTRL+C, shutting down.");
    Ok(())
}

// fn string_to_pubkey(input: &str) -> Result<Pubkey, Box<dyn std::error::Error>> {
//     let bytes = bs58::decode(input).into_vec()?;
//     let array: [u8; 32] = bytes.try_into().map_err(|_| "Invalid length")?;
//     Ok(Pubkey::from_str(&array))
// }
