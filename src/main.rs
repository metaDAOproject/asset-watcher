#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::prelude::*;
use futures::{stream, FutureExt, StreamExt, TryStreamExt};
use postgres::NoTls;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use std::{env, sync::Arc};
use tokio_postgres::{connect, AsyncMessage};
mod adapters;
mod entities;
mod entrypoints;
mod services;
use deadpool::managed::Object;
use deadpool_diesel::postgres::{Pool, Runtime};
use deadpool_diesel::Manager;
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
) -> Result<(), Box<dyn std::error::Error>> {
    entrypoints::jobs::transaction_indexing::run_job(pg_connection).await?;
    Ok(())
}

// TODO this should return a result
async fn setup_event_listeners(
    db_url: &str,
    managed_connection: Arc<Object<Manager<PgConnection>>>,
    pub_sub_client: Arc<PubsubClient>,
) {
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
                    task::spawn(entrypoints::events::token_accts_insert::new_handler(
                        n,
                        connect_clone,
                        Arc::clone(&pub_sub_client),
                    ));
                }
                "transactions_insert_channel" => {
                    task::spawn(entrypoints::events::transactions_insert::new_handler(
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO setup API and watchers before running backfill...
    env_logger::init();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pub_sub_client = adapters::rpc::get_pubsub_client().await?;
    let db = get_database_pool(&database_url).await?;

    let pub_sub_clone = Arc::clone(&pub_sub_client);
    let db_clone = Arc::clone(&db);

    run_jobs(Arc::clone(&db_clone)).await?;

    let database_url_copy = database_url.clone();
    task::spawn(async move { setup_event_listeners(&database_url_copy, db, pub_sub_client).await });

    // run the API
    entrypoints::http::routes::listen_and_serve(db_clone, pub_sub_clone).await;

    println!("Received CTRL+C, shutting down.");
    Ok(())
}
