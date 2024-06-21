use crate::entities::token_acct_balances::token_acct_balances;
use crate::entities::token_acct_balances::TokenAcctBalances;
use crate::entities::token_accts::token_accts;

use crate::entities::token_accts::TokenAcct;
use crate::entities::token_accts::TokenAcctStatus;
use crate::entities::tokens;
use crate::entities::transactions::Payload;
use chrono::Utc;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::PgConnection;
use serde_json::Value;
use solana_account_decoder::parse_account_data::ParsedAccount;
use solana_client::rpc_response::RpcResponseContext;
use std::io;
use std::io::ErrorKind;
use std::sync::Arc;

use super::transactions;

pub async fn handle_token_acct_change(
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    record: TokenAcct,
    updated_token_account: ParsedAccount,
    ctx: RpcResponseContext,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the object
    let parsed_object = updated_token_account.parsed.as_object();

    // Extract the token amount information
    let token_amount = parsed_object
        .and_then(|object| object.get("info"))
        .and_then(|info| info.get("tokenAmount"))
        .and_then(|token_amount| token_amount.as_object());

    // Bail out if token amount information is missing
    if token_amount.is_none() {
        println!("Could not parse token acct change");
        return Ok(());
    }

    let token_amount_unwrapped = token_amount.unwrap();

    // Extract amount and decimals, bail out if any are missing or not in expected format
    let new_amount_str = token_amount_unwrapped
        .get("amount")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            println!("amount not found or invalid");
            Box::new(io::Error::new(
                ErrorKind::InvalidData,
                "amount not found or invalid",
            ))
        })?;

    let new_amount: i64 = new_amount_str.parse().map_err(|_| {
        println!("Failed to parse amount as i64");
        Box::new(io::Error::new(
            ErrorKind::InvalidData,
            "Failed to parse amount as i64",
        ))
    })?;

    // Query the most recent value for the given token_acct to calculate the delta
    let record_clone = record.clone();
    let previous_balance = conn_manager
        .interact(move |conn| {
            token_acct_balances::table
                .filter(token_acct_balances::dsl::token_acct.eq(record_clone.token_acct.clone()))
                .order_by(token_acct_balances::dsl::slot.desc())
                .select(token_acct_balances::dsl::amount)
                .first::<i64>(conn)
                .optional()
        })
        .await??;

    let previous_balance = match previous_balance {
        Some(prev) => prev,
        None => 0,
    };

    let new_delta = new_amount - previous_balance;

    let new_balance = TokenAcctBalances {
        token_acct: record.token_acct.clone(),
        mint_acct: record.mint_acct.clone(),
        owner_acct: record.owner_acct.clone(),
        amount: new_amount,
        delta: new_delta,
        slot: ctx.slot as i64,
        created_at: Utc::now(),
        tx_sig: None,
    };

    let conn_manager_clone = conn_manager.clone();
    conn_manager_clone
        .interact(move |conn| {
            diesel::insert_into(token_acct_balances::table)
                .values(new_balance)
                .execute(conn)
        })
        .await??;

    let now = Utc::now();
    let mut token_balance: TokenAcct = record;
    token_balance.amount = new_amount;
    token_balance.updated_at = Some(now);

    conn_manager
        .interact(move |conn| {
            diesel::update(
                token_accts::table
                    .filter(token_accts::token_acct.eq(token_balance.token_acct.clone())),
            )
            .set(token_accts::amount.eq(new_amount))
            .execute(conn)
        })
        .await??;

    Ok(())
}

// TODO make this be able to run without updating token_acct to watching
pub async fn handle_token_acct_in_tx(
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    transaction_payload: Payload,
    transaction_sig: String,
    mint_acct_value: &str,
    token_account: &str,
    authority_account: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mint_acct_value_str = mint_acct_value.to_string();
    let token_account_str = token_account.to_string();
    let authority_account_str = authority_account.to_string();
    let transaction_sig_str = transaction_sig.clone();

    // Check if the token record exists
    let mint_acct_value_clone = mint_acct_value_str.clone();
    let token_record_exists = conn_manager
        .interact(move |db| {
            tokens::tokens::table
                .filter(tokens::tokens::dsl::mint_acct.eq(mint_acct_value_clone))
                .count()
                .get_result::<i64>(db)
        })
        .await??
        > 0;

    if !token_record_exists {
        println!(
            "Token table record not found for account: {}",
            token_account_str
        );
        return Ok(());
    }

    // Find the matching account in the root accounts array and extract postBalance
    let account_with_balance = transaction_payload
        .accounts
        .iter()
        .find(|acc| acc.pubkey == token_account_str)
        .ok_or("Matching account not found in transaction payload")?;

    let account_balance = match &account_with_balance.post_token_balance {
        Some(token_balance) => token_balance
            .amount
            .split(':')
            .nth(1)
            .ok_or("Invalid postBalance format")?
            .parse::<i64>()?,
        None => 0,
    };

    // Check if the token account already exists
    let token_account_clone = token_account_str.clone();
    let token_acct_record: Vec<TokenAcct> = conn_manager
        .interact(move |db| {
            token_accts::table
                .filter(token_accts::dsl::token_acct.eq(token_account_clone))
                .load::<TokenAcct>(db)
        })
        .await??;

    if token_acct_record.is_empty() {
        let new_token_acct = TokenAcct {
            token_acct: token_account_str.clone(),
            owner_acct: authority_account_str.clone(),
            amount: account_balance,
            status: TokenAcctStatus::Watching,
            mint_acct: mint_acct_value_str.clone(),
            updated_at: Some(Utc::now()),
        };

        let new_token_acct_clone = new_token_acct.clone();
        conn_manager
            .interact(move |db| {
                diesel::insert_into(token_accts::table)
                    .values(&new_token_acct_clone)
                    .execute(db)
            })
            .await??;
    } else {
        let token_account_update = token_account_str.clone();
        conn_manager
            .interact(move |db| {
                diesel::update(
                    token_accts::table.filter(token_accts::token_acct.eq(token_account_update)),
                )
                .set(token_accts::status.eq(TokenAcctStatus::Watching))
                .execute(db)
            })
            .await??;
    }

    transactions::handle_token_acct_balance_tx(
        conn_manager.clone(),
        token_account_str.clone(),
        account_balance,
        transaction_sig_str,
        transaction_payload.slot,
        mint_acct_value_str,
        authority_account_str,
    )
    .await?;

    Ok(())
}
