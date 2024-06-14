use diesel::prelude::*;
use diesel::PgConnection;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;

use crate::entities::token_acct_balances::token_acct_balances;
use crate::entities::token_acct_balances::TokenAcctBalances;
use crate::entities::token_accts::token_accts;
use crate::events;

/**
 * Handles updating our DB for a tx that affects a token acct balance.
 */
pub async fn handle_token_acct_balance_tx(
    connection: &mut PgConnection,
    pub_sub_client: Arc<PubsubClient>,
    token_acct: String,
    new_balance: i64,
    transaction_sig: String,
    slot: i64,
    create_acct_subscriber: bool,
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
    let balance_exists = token_acct_balances::table
        .filter(
            token_acct_balances::slot
                .eq(slot)
                .and(token_acct_balances::token_acct.eq(token_acct.clone())),
        )
        .count()
        .get_result::<i64>(connection)?
        > 0;

    if balance_exists {
        // Update the tx_sig field if the balance record exists
        diesel::update(
            token_acct_balances::table.filter(
                token_acct_balances::token_acct
                    .eq(token_acct.clone())
                    .and(token_acct_balances::slot.eq(slot)),
            ),
        )
        .set(token_acct_balances::tx_sig.eq(transaction_sig.clone()))
        .execute(connection)?;
    } else {
        // Insert a new balance record if it does not exist
        let new_token_acct_balance = TokenAcctBalances {
            token_acct: token_acct.clone(),
            mint_acct: "".to_string(), // Placeholder, as it is not provided
            owner_acct: "".to_string(), // Placeholder, as it is not provided
            amount: new_balance,
            delta,
            slot: slot,
            tx_sig: Some(transaction_sig.clone()),
            created_at: SystemTime::now(),
        };

        diesel::insert_into(token_acct_balances::table)
            .values(&new_token_acct_balance)
            .execute(connection)?;
    }

    if create_acct_subscriber {
        let token_acct_record = token_accts::table
            .filter(token_accts::dsl::token_acct.eq(&token_acct))
            .first(connection)?;
        let pub_key = Pubkey::from_str(&token_acct)?;
        events::rpc_token_acct_updates::new_handler(
            pub_sub_client,
            connection,
            pub_key,
            token_acct_record,
        )
        .await;
    }

    Ok(())
}
