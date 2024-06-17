#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::prelude::*;
use dotenv::dotenv;
use entities::token_accts::TokenAcctStatus;
use futures::executor::block_on;
use futures::{stream, FutureExt, StreamExt, TryStreamExt};
use postgres::NoTls;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_program::pubkey::Pubkey;
use std::{env, sync::Arc};
use tokio::signal;
use tokio_postgres::{connect, AsyncMessage};
mod entities;
mod jobs;
use entities::token_accts::{token_accts::dsl::*, TokenAcct};
mod adapters;
mod events;
mod services;
use deadpool::managed::Object;
use deadpool_diesel::postgres::{Pool, Runtime};
use deadpool_diesel::Manager;
use std::str::FromStr;
use tokio::task::{self};

async fn get_database_pool(
    db_url: &str,
) -> Result<Arc<Object<Manager<PgConnection>>>, Box<dyn std::error::Error>> {
    let manager = Manager::new(db_url, Runtime::Tokio1);
    let pool = Pool::builder(manager).max_size(8).build()?;
    let conn_manager = pool.get().await?;
    Ok(Arc::new(conn_manager))
}

async fn run_jobs(
    pg_connection: Arc<Object<Manager<PgConnection>>>,
    rpc_pub_sub_client: Arc<PubsubClient>,
) -> Result<(), Box<dyn std::error::Error>> {
    jobs::transaction_indexing::run_job(pg_connection, rpc_pub_sub_client).await?;
    Ok(())
}

// TODO this should return a result
async fn setup_event_listeners(
    db_url: &str,
    managed_connection: Arc<Object<Manager<PgConnection>>>,
    pub_sub_client: Arc<PubsubClient>,
) -> Result<(), Box<dyn std::error::Error>> {
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
                    conn_manager_arg_clone
                        .interact(move |conn| {
                            println!(
                                "about to subscribe to acct: {}",
                                token_acct_pubkey.to_string()
                            );
                            block_on(events::rpc_token_acct_updates::new_handler(
                                pub_sub_client_clone,
                                conn,
                                token_acct_pubkey,
                                record,
                            ))
                        })
                        .await
                });
            }
            Err(e) => eprintln!("Error with token acct pubkey parsing: {}", e),
        }
    }

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
        LISTEN transactions_insert_channel;
        LISTEN token_accts_insert_channel;
    ",
        )
        .await
        .unwrap();

    while let Some(m) = rx.next().await {
        let connect_clone = Arc::clone(&managed_connection);
        match m {
            AsyncMessage::Notification(n) => match n.channel() {
                "token_accts_insert_channel" => {
                    task::spawn(events::token_accts_insert::new_handler(
                        n,
                        connect_clone,
                        Arc::clone(&pub_sub_client),
                    ));
                }
                "transactions_insert_channel" => {
                    task::spawn(events::transactions_insert::new_handler(
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

    // TODO: event handlers should not return results themselves.. but things they call will return results and they should handle that internally

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pub_sub_client = adapters::rpc::get_pubsub_client().await?;
    let conn_manager_arc = get_database_pool(&database_url).await?;
    run_jobs(Arc::clone(&conn_manager_arc), Arc::clone(&pub_sub_client)).await?;

    setup_event_listeners(&database_url, conn_manager_arc, pub_sub_client).await?;

    // Block the main function and handle CTRL+C
    signal::ctrl_c().await?;
    println!("Received CTRL+C, shutting down.");
    Ok(())
}
