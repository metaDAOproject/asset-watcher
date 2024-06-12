#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::prelude::*;
use dotenv::dotenv;
use entities::token_accts::TokenAcctStatus;
use events::rpc_token_acct_updates;
use solana_program::pubkey::Pubkey;
use std::{env, sync::Arc};
use tokio::signal;
mod entities;
use entities::token_accts::{token_accts::dsl::*, TokenAcct};
mod adapters;
mod events;
mod services;
use deadpool::managed::Object;
use deadpool_diesel::postgres::{Pool, Runtime};
use deadpool_diesel::Manager;
use std::str::FromStr;

async fn get_database_pool(
    db_url: &str,
) -> Result<Arc<Object<Manager<PgConnection>>>, Box<dyn std::error::Error>> {
    let manager = Manager::new(db_url, Runtime::Tokio1);
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

// TODO this should return a result
async fn setup_event_listeners(
    db_url: &str,
    managed_connection: Arc<Object<Manager<PgConnection>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pub_sub_client = get_pubsub_client().await?;

    let results = managed_connection
        .clone()
        .interact(|conn| {
            return token_accts
                .filter(status.eq(TokenAcctStatus::Watching))
                .load::<TokenAcct>(conn)
                .expect("Error loading token_accts");
        })
        .await?;

    for record in results {
        match Pubkey::from_str(&record.token_acct) {
            Ok(token_acct_pubkey) => {
                let conn_manager_arg_clone = Arc::clone(&managed_connection);
                let pub_sub_client_clone = Arc::clone(&pub_sub_client);
                tokio::spawn(async move {
                    rpc_token_acct_updates::new_handler(
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
    events::transactions_insert::new_handler(db_url, Arc::clone(&managed_connection)).await?;
    events::token_accts_insert::new_handler(db_url, managed_connection, pub_sub_client).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let conn_manager_arc = get_database_pool(&database_url).await?;
    setup_event_listeners(&database_url, Arc::clone(&conn_manager_arc)).await?;

    // Block the main function and handle CTRL+C
    signal::ctrl_c().await?;
    println!("Received CTRL+C, shutting down.");
    Ok(())
}
