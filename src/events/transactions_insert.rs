use crate::entities::transactions::{InstructionType, Payload, TransactionsInsertChannelPayload};
use crate::services;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::{ExpressionMethods, PgConnection};
use futures::executor::block_on;
use postgres::Notification;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use std::sync::Arc;

use crate::entities::transactions::{transactions::dsl::*, Transaction};

pub async fn new_handler(
    notification: Notification,
    pool_connection: Arc<Object<Manager<PgConnection>>>,
    pub_sub_client: Arc<PubsubClient>,
) {
    println!(
        "new transactions table payload: {:?}",
        notification.payload()
    );
    match TransactionsInsertChannelPayload::parse_payload(notification.payload()) {
        Ok(tx_payload) => {
            match handle_new_transaction(tx_payload.tx_sig, pool_connection, pub_sub_client).await {
                Ok(()) => println!("successfully handled new transaction notification"),
                Err(e) => eprintln!("error handling new transaction notification: {:?}", e),
            }
        }
        Err(e) => eprintln!("error parsing new transaction notification: {:?}", e),
    };
}

async fn handle_new_transaction(
    transaction_signature: String,
    connection: Arc<Object<Manager<PgConnection>>>,
    pub_sub_client: Arc<PubsubClient>,
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
                    .interact(move |conn| {
                        let mint_handler_res = services::new_mint::handle_mint_tx(
                            conn,
                            pub_sub_client,
                            payload_parsed.clone(),
                            txn[0].tx_sig.clone(),
                        );

                        let mint_handler_res_awaited = block_on(mint_handler_res);
                        match mint_handler_res_awaited {
                            Ok(_) => println!(
                                "handled new mint tx: {:?}, {:?}",
                                payload_parsed.signatures,
                                payload_parsed.get_main_ix_type()
                            ),
                            Err(e) => eprintln!(
                                "error tracking new mint: {:?}. payload: {:?}",
                                e, payload_parsed
                            ),
                        }
                    })
                    .await?;
            }
            InstructionType::AmmSwap => {
                connection
                    .interact(move |conn| {
                        let mint_handler_res = services::swaps::handle_swap_tx(
                            conn,
                            pub_sub_client,
                            payload_parsed.clone(),
                            txn[0].tx_sig.clone(),
                        );

                        let mint_handler_res_awaited = block_on(mint_handler_res);
                        match mint_handler_res_awaited {
                            Ok(_) => println!(
                                "handled new mint tx: {:?}, {:?}",
                                payload_parsed.signatures,
                                payload_parsed.get_main_ix_type()
                            ),
                            Err(e) => eprintln!(
                                "error tracking new mint: {:?}. payload: {:?}",
                                e, payload_parsed
                            ),
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
