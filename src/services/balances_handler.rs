use crate::entities::token_acct_balances::token_acct_balances;
use crate::entities::token_acct_balances::TokenAcctBalancesRecord;
use crate::entities::token_accts::token_accts;
use crate::entities::token_accts::token_accts::dsl::*;
use crate::entities::token_accts::TokenAcct;
use diesel::prelude::*;
use diesel::PgConnection;
use solana_account_decoder::parse_account_data::ParsedAccount;
use std::io::ErrorKind;
use std::time::SystemTime;

pub fn handle_token_acct_change(
    connection: &mut PgConnection,
    record: TokenAcct,
    updated_token_account: ParsedAccount,
) -> Result<(), ErrorKind> {
    let token_amount = updated_token_account
        .parsed
        .as_object()
        .unwrap()
        .get("info")
        .unwrap()
        .get("tokenAmount")
        .unwrap();
    let new_amount = token_amount.get("uiAmount").unwrap().as_f64().unwrap();
    let decimals = token_amount.get("decimals").unwrap().as_f64().unwrap();

    let amount_scaled = new_amount * 10f64.powf(decimals);

    let new_balance = TokenAcctBalancesRecord {
        token_acct: record.token_acct.clone(),
        mint_acct: record.mint_acct.clone(),
        owner_acct: record.owner_acct.clone(),
        amount: amount_scaled,
        created_at: SystemTime::now(),
    };

    diesel::insert_into(token_acct_balances::table)
        .values(new_balance)
        .execute(connection)
        .expect("Error inserting into token_acct_balances");

    let now = SystemTime::now();
    let mut token_balance: TokenAcct = record;
    token_balance.amount = amount_scaled;
    token_balance.updated_at = Some(now);
    diesel::update(token_accts::table.filter(token_acct.eq(token_balance.token_acct.clone())))
        .set(amount.eq(amount_scaled))
        .execute(connection)
        .expect("Error updating token_accts");
    Ok(())
}
