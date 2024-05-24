#[macro_use]
extern crate diesel;
extern crate dotenv;

use adapters::rpc::SolanaRpcClient;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
mod entities;
use entities::token_accts::{token_accts::dsl::*, TokenAcct};
mod adapters;
mod services;
use deadpool_diesel::postgres::{Manager, Pool, Runtime};
// use services::balances_handler::BalancesHandler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = Manager::new(database_url, Runtime::Tokio1);
    let pool = Pool::builder(manager).max_size(8).build()?;
    let conn_manager = pool.get().await?;
    // let other_connection = &mut establish_connection();

    let rpc_endpoint_ws = env::var("RPC_ENDPOINT_WSS").expect("RPC_ENDPOINT_WSS must be set");
    let rpc_client = SolanaRpcClient::new(&rpc_endpoint_ws).await?;
    let rpc_client_arc = Arc::new(Mutex::new(rpc_client));
    // let arc_connection = Arc::new(*connection);
    // let balances_handler = BalancesHandler::new(arc_connection);

    let results = conn_manager
        .interact(|conn| {
            return token_accts
                .load::<TokenAcct>(conn)
                .expect("Error loading token_accts");
        })
        .await?;

    // let results = token_accts
    //     .load::<TokenAcct>(other_connection)
    //     .expect("Error loading token_accts");

    for record in results {
        let rpc_clone = Arc::clone(&rpc_client_arc);
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            let token_acct_clone = record.token_acct.clone();
            let mut rpc_client = rpc_clone.lock().await;
            rpc_client
                .on_account_change(token_acct_clone.clone(), move |msg| {
                    let parsed_msg: serde_json::Value =
                        serde_json::from_str(msg.as_str()).expect("Failed to parse JSON");
                    let new_amount = parsed_msg["params"]["result"]["value"]["amount"]
                        .as_i64()
                        .expect("Failed to get amount");

                    let pool_clone_inner = pool_clone.clone();
                    let token_acct_inner = token_acct_clone.clone();
                    tokio::spawn(async move {
                        let new_conn_inner = pool_clone_inner
                            .get()
                            .await
                            .expect("Failed to get connection from pool");
                        let update_res = new_conn_inner
                            .interact(move |conn| {
                                diesel::update(token_accts.filter(token_acct.eq(token_acct_inner)))
                                    .set(amount.eq(new_amount))
                                    .execute(conn)
                            })
                            .await
                            .expect("Failed to update token_accts");

                        println!("updated token_accts = {:?}", update_res);
                    });
                })
                .await;
        });
    }
    Ok(())
}

pub fn establish_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
