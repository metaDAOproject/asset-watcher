use chrono::Utc;
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
    connection: &mut PgConnection,
    token_acct: String,
    new_balance: i64,
    transaction_sig: String,
    slot: i64,
    mint_acct: String,
    owner_acct: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Query the most recent value for the given token_acct to calculate the delta
    let previous_balance = token_acct_balances::table
        .filter(token_acct_balances::token_acct.eq(token_acct.clone()))
        .order(token_acct_balances::slot.desc())
        .select(token_acct_balances::amount)
        .first::<i64>(connection)
        .optional()?;

    let delta = match previous_balance {
        Some(prev_amount) => new_balance - prev_amount,
        None => new_balance, // If no previous balance exists, delta is the new_balance itself
    };

    // Check if the balance record exists for the specific slot and token account
    let existing_balance_res: Result<TokenAcctBalances, diesel::result::Error> =
        token_acct_balances::table
            .filter(
                token_acct_balances::slot
                    .eq(slot)
                    .and(token_acct_balances::token_acct.eq(token_acct.clone())),
            )
            .first(connection);

    let maybe_balance = existing_balance_res.ok();
    let maybe_balance_copy = maybe_balance.clone();

    if maybe_balance.is_some() && maybe_balance.unwrap().tx_sig.is_none() {
        // Update the tx_sig field if the balance record exists and has no tx_sig
        diesel::update(
            token_acct_balances::table.filter(
                token_acct_balances::token_acct
                    .eq(token_acct.clone())
                    .and(token_acct_balances::slot.eq(slot)),
            ),
        )
        .set(token_acct_balances::tx_sig.eq(transaction_sig.clone()))
        .execute(connection)?;
    }
    if maybe_balance_copy.is_none() {
        // Insert a new balance record if it does not exist
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

        diesel::insert_into(token_acct_balances::table)
            .values(&new_token_acct_balance)
            .execute(connection)?;
    }

    Ok(())
}
