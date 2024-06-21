use std::sync::Arc;

use chrono::Utc;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::PgConnection;

use crate::entities::token_acct_balances::token_acct_balances;
use crate::entities::token_acct_balances::TokenAcctBalances;
// use crate::entrypoints::events;

/**
 * Handles updating our DB for a tx that affects a token acct balance.
 */
//TODO: create status Watching for updates on tx
pub async fn handle_token_acct_balance_tx(
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    token_acct: String,
    new_balance: i64,
    transaction_sig: String,
    slot: i64,
    mint_acct: String,
    owner_acct: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Query the most recent value for the given token_acct to calculate the delta
    let token_acct_clone_1 = token_acct.clone();
    let previous_balance = conn_manager
        .interact(move |db| {
            token_acct_balances::table
                .filter(token_acct_balances::token_acct.eq(token_acct_clone_1))
                .order(token_acct_balances::slot.desc())
                .select(token_acct_balances::amount)
                .first::<i64>(db)
                .optional()
        })
        .await??;

    let delta = match previous_balance {
        Some(prev_amount) => new_balance - prev_amount,
        None => new_balance,
    };

    let token_acct_clone_2 = token_acct.clone();
    let existing_balance_res = conn_manager
        .interact(move |db| {
            token_acct_balances::table
                .filter(
                    token_acct_balances::slot
                        .eq(slot)
                        .and(token_acct_balances::token_acct.eq(token_acct_clone_2)),
                )
                .first::<TokenAcctBalances>(db)
        })
        .await?;

    let maybe_balance = existing_balance_res.ok();
    let maybe_balance_copy = maybe_balance.clone();

    if let Some(balance) = maybe_balance {
        if balance.tx_sig.is_none() {
            let token_acct_clone_3 = token_acct.clone();
            let tx_sig_clone = transaction_sig.clone();
            conn_manager
                .interact(move |db| {
                    diesel::update(
                        token_acct_balances::table.filter(
                            token_acct_balances::token_acct
                                .eq(token_acct_clone_3)
                                .and(token_acct_balances::slot.eq(slot)),
                        ),
                    )
                    .set(token_acct_balances::tx_sig.eq(tx_sig_clone))
                    .execute(db)
                })
                .await??;
        }
    }

    if maybe_balance_copy.is_none() {
        let new_token_acct_balance = TokenAcctBalances {
            token_acct: token_acct.clone(),
            mint_acct: mint_acct,
            owner_acct: owner_acct,
            amount: new_balance,
            delta,
            slot: slot,
            tx_sig: Some(transaction_sig.clone()),
            created_at: Utc::now(),
        };

        conn_manager
            .interact(move |db| {
                diesel::insert_into(token_acct_balances::table)
                    .values(&new_token_acct_balance)
                    .execute(db)
            })
            .await??;
    }

    Ok(())
}
