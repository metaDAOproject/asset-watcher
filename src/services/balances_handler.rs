use crate::entities::token_accts::{token_accts::dsl::*, TokenAcct};
use diesel::prelude::*;
use diesel::PgConnection;
use serde_json::Value;
use std::sync::Arc;
use std::time::SystemTime;

pub struct BalancesHandler {
    pub connection: Arc<PgConnection>,
}

impl BalancesHandler {
    pub fn new(connection: Arc<PgConnection>) -> Self {
        BalancesHandler { connection }
    }

    pub fn handle_token_acct_change(&self, record: TokenAcct, msg: &str) {
        // let parsed_msg: Value = serde_json::from_str(msg).expect("Failed to parse JSON");
        // let new_amount = parsed_msg["params"]["result"]["value"]["amount"]
        //     .as_i64()
        //     .expect("Failed to get amount");

        // let now = SystemTime::now();

        // let connection_clone = Arc::clone(&self.connection);
        // diesel::update(token_accts.filter(token_acct.eq(record.token_acct)))
        //     .set(amount.eq(new_amount))
        //     .execute(&mut *connection_clone);

        // Insert a new row into the TokenAcctBalances table
        // let new_balance = NewTokenAcctBalances {
        //     token_acct: record.token_acct.clone(),
        //     mint_acct: record.mint_acct.clone(),
        //     owner_acct: record.owner_acct.clone(),
        //     amount: new_amount,
        //     created_at: SystemTime::now(),
        // };

        // diesel::insert_into(token_acct_balances::table)
        //     .values(&new_balance)
        //     .execute(&self.connection)
        //     .expect("Error inserting into token_acct_balances");
    }
}
