use std::sync::Arc;

use chrono::{Duration, NaiveDateTime, Utc};
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::{ExpressionMethods, PgConnection};

use crate::entities::transactions::transactions;
use crate::entities::transactions::transactions::main_ix_type;
use crate::entities::transactions::{transactions::block_time, Transaction};
use crate::events;
use diesel::prelude::*;

pub async fn run_job(
    pg_connection: Arc<Object<Manager<PgConnection>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let thirty_days_ago = Utc::now().naive_utc() - Duration::days(5);

    // Run the query
    let transactions =
        get_recent_transactions_with_main_ix_type(thirty_days_ago, Arc::clone(&pg_connection))
            .await?;

    // Process each transaction
    for transaction in transactions {
        let pg_clone = Arc::clone(&pg_connection);
        events::transactions_insert::index_tx_record(transaction, pg_clone, None).await?;
    }
    Ok(())
}

async fn get_recent_transactions_with_main_ix_type(
    thirty_days_ago: NaiveDateTime,
    connection: Arc<Object<Manager<PgConnection>>>,
) -> Result<Vec<Transaction>, Box<dyn std::error::Error>> {
    let res = connection
        .interact(move |conn| {
            transactions::table
                .filter(
                    main_ix_type
                        .is_not_null()
                        .and(block_time.ge(thirty_days_ago)),
                )
                .load::<Transaction>(conn)
        })
        .await?;

    Ok(res?)
}
