use crate::entities::transactions::{InstructionType, Payload, TransactionsInsertChannelPayload};
use crate::services;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::{ExpressionMethods, PgConnection};
use futures::executor::block_on;
use postgres::Notification;
use std::sync::Arc;

use crate::entities::transactions::{transactions::dsl::*, Transaction};

pub async fn new_handler(
    notification: Notification,
    pool_connection: Arc<Object<Manager<PgConnection>>>,
) {
    println!(
        "new transactions table payload: {:?}",
        notification.payload()
    );
    match TransactionsInsertChannelPayload::parse_payload(notification.payload()) {
        Ok(tx_payload) => match handle_new_transaction(tx_payload.tx_sig, pool_connection).await {
            Ok(()) => println!("successfully handled new transaction notification"),
            Err(e) => eprintln!("error handling new transaction notification: {:?}", e),
        },
        Err(e) => eprintln!("error parsing new transaction notification: {:?}", e),
    };
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

    let txn_vec: Vec<Transaction> = txn_result?;
    let txn = &txn_vec[0];

    index_tx_record(txn.clone(), connection).await?;

    Ok(())
}

pub async fn index_tx_record(     
    tx: Transaction,
    connection: Arc<Object<Manager<PgConnection>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload_parsed = Payload::parse_payload(&tx.payload)?;

    match payload_parsed.get_main_ix_type() {
        Some(ix_type) => match ix_type {
            InstructionType::VaultMintConditionalTokens => {
                let mint_handler_res = services::new_mint::handle_mint_tx(
                    connection,
                    &payload_parsed,
                    tx.tx_sig.clone(),
                );

                let mint_handler_res_awaited = block_on(mint_handler_res);
                match mint_handler_res_awaited {
                    Ok(_) => println!(
                        "handled new mint tx: {:?}, {:?}",
                        payload_parsed.signatures,
                        payload_parsed.get_main_ix_type()
                    ),
                    Err(e) => eprintln!(
                        "error tracking new mint: {:?}. payload instructions: {:?}",
                        e, payload_parsed.instructions
                    ),
                }
            }
            InstructionType::AmmSwap => {
                let swap_res = services::swaps::handle_swap_tx(
                    connection,
                    &payload_parsed,
                    tx.tx_sig.clone(),
                );

                let swap_res_awaited = block_on(swap_res);
                match swap_res_awaited {
                    Ok(_) => println!(
                        "handled swap tx: {:?}, {:?}",
                        payload_parsed.signatures,
                        payload_parsed.get_main_ix_type()
                    ),
                    Err(e) => eprintln!(
                        "error tracking swap: {:?}. payload: {:?}",
                        e, payload_parsed
                    ),
                }
            }
            InstructionType::AmmDeposit => {
                let amm_deposit_res = services::liquidity::handle_lp_deposit_tx(
                    connection,
                    &payload_parsed,
                    tx.tx_sig.clone(),
                );

                let amm_deposit_res_awaited = block_on(amm_deposit_res);
                match amm_deposit_res_awaited {
                    Ok(_) => println!(
                        "handled amm deposit tx: {:?}, {:?}",
                        payload_parsed.signatures,
                        payload_parsed.get_main_ix_type()
                    ),
                    Err(e) => eprintln!(
                        "error tracking amm deposit: {:?}. payload: {:?}",
                        e, payload_parsed
                    ),
                }
            }
            InstructionType::AmmWithdraw => {
                let amm_withdrawal_res = services::liquidity::handle_lp_withdrawal_tx(
                    connection,
                    &payload_parsed,
                    tx.tx_sig.clone(),
                );

                let amm_withdrawal_res_awaited = block_on(amm_withdrawal_res);
                match amm_withdrawal_res_awaited {
                    Ok(_) => println!(
                        "handled amm withdrawal tx: {:?}, {:?}",
                        payload_parsed.signatures,
                        payload_parsed.get_main_ix_type()
                    ),
                    Err(e) => eprintln!(
                        "error tracking amm withdrawal: {:?}. payload: {:?}",
                        e, payload_parsed
                    ),
                }
            }
            InstructionType::VaultMergeConditionalTokens => {
                let merge_conditionals_res =
                    services::merge_conditionals_for_underlying::handle_merge_conditional_tokens_tx(
                        connection,
                        &payload_parsed,
                        tx.tx_sig.clone(),
                    );

                let merge_conditionals_res_awaited = block_on(merge_conditionals_res);
                match merge_conditionals_res_awaited {
                    Ok(_) => println!(
                        "handled merge conditionals tx: {:?}, {:?}",
                        payload_parsed.signatures,
                        payload_parsed.get_main_ix_type()
                    ),
                    Err(e) => eprintln!(
                        "error tracking merge conditionals: {:?}. payload: {:?}",
                        e, payload_parsed
                    ),
                }
            }
            InstructionType::VaultRedeemConditionalTokensForUnderlyingTokens => {
                let redeem_conditionals_res =
                    services::redeem_conditionals::handle_redeem_conditional_tokens_tx(
                        connection,
                        &payload_parsed,
                        tx.tx_sig.clone(),
                    );

                let redeem_conditionals_res_awaited = block_on(redeem_conditionals_res);
                match redeem_conditionals_res_awaited {
                    Ok(_) => println!(
                        "handled redeem conditionals tx: {:?}, {:?}",
                        payload_parsed.signatures,
                        payload_parsed.get_main_ix_type()
                    ),
                    Err(e) => eprintln!(
                        "error tracking redeem conditionals: {:?}. payload: {:?}",
                        e, payload_parsed
                    ),
                }
            }
            x => println!("unhandled ix type: {:?}", x),
        },
        None => println!("tx has no ix type we care about"),
    }

    Ok(())
}
