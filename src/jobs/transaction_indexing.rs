use std::sync::Arc;

use chrono::{Duration, NaiveDateTime, Utc};
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::{ExpressionMethods, PgConnection};
use solana_client::nonblocking::pubsub_client::PubsubClient;

use crate::entities::transactions::transactions;
use crate::entities::transactions::transactions::main_ix_type;
use crate::entities::transactions::{transactions::block_time, Transaction};
use crate::events;
use diesel::prelude::*;

pub async fn run_job(
    connection: Arc<Object<Manager<PgConnection>>>,
    pub_sub_client: Arc<PubsubClient>,
) {
    let thirty_days_ago = Utc::now().naive_utc() - Duration::days(30);

    // Run the query
    let transactions = get_recent_transactions_with_main_ix_type(thirty_days_ago, &connection)
        .expect("Failed to query transactions");

    // Process each transaction
    for transaction in transactions {
        events::transactions_insert::index_tx_record(
            transaction,
            Arc::clone(&connection),
            Arc::clone(&pub_sub_client),
        )
        .await;
    }
}

async fn get_recent_transactions_with_main_ix_type(
    thirty_days_ago: NaiveDateTime,
    connection: Arc<Object<Manager<PgConnection>>>,
) -> QueryResult<Vec<Transaction>> {
    let res = connection
        .interact(move |conn| {
            transactions::table
                .filter(
                    main_ix_type
                        .is_not_null()
                        .and(block_time.ge(thirty_days_ago)),
                )
                .load::<Transaction>(&conn)
        })
        .await?;

    res
}
