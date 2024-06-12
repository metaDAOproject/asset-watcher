use crate::entities::transactions::{InstructionType, Payload, TransactionsInsertChannelPayload};
use crate::services;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::{ExpressionMethods, PgConnection};
use futures::{stream, FutureExt, TryStreamExt};
use futures_util::StreamExt;
use postgres::NoTls;
use std::sync::Arc;
use tokio_postgres::{connect, AsyncMessage};

use crate::entities::transactions::{transactions::dsl::*, Transaction};

// TODO this should just accept client instead of pool_connection mayb
pub async fn new_handler(
    db_url: &str,
    pool_connection: Arc<Object<Manager<PgConnection>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (client, mut connection) = connect(db_url, NoTls).await.unwrap();
    // Make transmitter and receiver.
    let (tx, mut rx) = futures_channel::mpsc::unbounded();
    let stream =
        stream::poll_fn(move |cx| connection.poll_message(cx)).map_err(|e| panic!("{}", e));
    let connection = stream.forward(tx).map(|r| r.unwrap());
    tokio::spawn(connection);

    client
        .batch_execute("LISTEN transactions_insert_channel;")
        .await
        .unwrap();

    while let Some(m) = rx.next().await {
        match m {
            AsyncMessage::Notification(n) => match n.channel() {
                "transactions_insert_channel" => {
                    println!("new transactions table payload: {:?}", n.payload());
                    let cloned_connection = Arc::clone(&pool_connection);
                    let tx_payload =
                        TransactionsInsertChannelPayload::parse_payload(n.payload()).unwrap();
                    handle_new_transaction(tx_payload.tx_sig, cloned_connection).await?;
                }
                _ => println!("unhandled channel: {:?}", n.payload()),
            },
            AsyncMessage::Notice(notice) => println!("async message error: {:?}", notice),
            _ => println!("fallthrough handler of async message from postgres listener"),
        }
    }
    Ok(())
}

async fn handle_new_transaction(
    transaction_signature: String,
    connection: Arc<Object<Manager<PgConnection>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let txn_result = connection
        .clone()
        .interact(|conn| {
            return transactions
                .filter(tx_sig.eq(transaction_signature))
                .limit(1)
                .select(Transaction::as_select())
                .load(conn);
        })
        .await?;

    let txn = txn_result?;

    let payload_parsed = Payload::parse_payload(&txn[0].payload)?;

    match payload_parsed.get_main_ix_type() {
        Some(ix_type) => match ix_type {
            InstructionType::VaultMintConditionalTokens => {
                connection
                    .interact(|conn| {
                        let mint_handler_res =
                            services::new_mint_handlers::handle_mint_tx(conn, payload_parsed);
                        match mint_handler_res {
                            Ok(_) => println!("handled new mint tx"),
                            Err(e) => eprintln!("error tracking new mint: {:?}", e),
                        }
                    })
                    .await?;
            }
            x => println!("unhandled ix type: {:?}", x),
        },
        None => println!("tx has no ix type we care about"),
    }

    Ok(())
}
