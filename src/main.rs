#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::prelude::*;
use std::{env, sync::Arc};
use tokio::signal;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pub_sub_client = adapters::rpc::get_pubsub_client().await?;
    let db = get_database_pool(&database_url).await?;

    let db_clone = Arc::clone(&db);
    let db_clone_2 = Arc::clone(&db);

    let database_url_copy = database_url.clone();
    task::spawn(async move {
        entrypoints::events::setup::setup_event_listeners(&database_url_copy, db, pub_sub_client)
            .await
    });

    // run the API
    task::spawn(async move { entrypoints::http::routes::listen_and_serve(db_clone).await });

    // TODO setup API and watchers before running backfill...
    run_jobs(db_clone_2).await?;

    signal::ctrl_c().await?;
    println!("Received CTRL+C, shutting down.");
    Ok(())
}
