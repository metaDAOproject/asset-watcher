use crate::entities::token_acct_balances::token_acct_balances;
use crate::entities::token_acct_balances::TokenAcctBalancesRecord;
use crate::entities::token_accts::token_accts;
use crate::entities::token_accts::token_accts::dsl::*;
use crate::entities::token_accts::TokenAcct;
use diesel::prelude::*;
use diesel::PgConnection;
use serde_json::Value;
use solana_account_decoder::parse_account_data::ParsedAccount;
use std::io::ErrorKind;
use std::time::SystemTime;

pub fn handle_token_acct_change(
    connection: &mut PgConnection,
    record: TokenAcct,
    updated_token_account: ParsedAccount,
) -> Result<(), ErrorKind> {
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
            ErrorKind::InvalidData
        })?;

    let new_amount: i64 = new_amount_str.parse().map_err(|_| {
        println!("Failed to parse amount as i64");
        ErrorKind::InvalidData
    })?;

    let new_balance = TokenAcctBalancesRecord {
        token_acct: record.token_acct.clone(),
        mint_acct: record.mint_acct.clone(),
        owner_acct: record.owner_acct.clone(),
        amount: new_amount,
        created_at: SystemTime::now(),
    };

    diesel::insert_into(token_acct_balances::table)
        .values(new_balance)
        .execute(connection)
        .expect("Error inserting into token_acct_balances");

    let now = SystemTime::now();
    let mut token_balance: TokenAcct = record;
    token_balance.amount = new_amount;
    token_balance.updated_at = Some(now);
    diesel::update(token_accts::table.filter(token_acct.eq(token_balance.token_acct.clone())))
        .set(amount.eq(new_amount))
        .execute(connection)
        .expect("Error updating token_accts");
    Ok(())
}
