use crate::entities::token_acct_balances::token_acct_balances;
use crate::entities::token_acct_balances::TokenAcctBalancesRecord;
use crate::entities::token_accts::token_accts;
use crate::entities::token_accts::token_accts::dsl::*;
use crate::entities::token_accts::TokenAcct;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::PgConnection;
use std::time::SystemTime;

pub async fn handle_token_acct_change(
    connection: Object<Manager<PgConnection>>,
    record: TokenAcct,
    new_amount: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    // let parsed_msg: Value = serde_json::from_str(msg).expect("Failed to parse JSON");
    // let new_amount = parsed_msg["params"]["result"]["value"]["amount"]
    //     .as_i64()
    //     .expect("Failed to get amount");

    // let now = SystemTime::now();

    // let connection_clone = Arc::clone(&self.connection);

    // Insert a new row into the TokenAcctBalances table
    let new_balance = TokenAcctBalancesRecord {
        token_acct: record.token_acct.clone(),
        mint_acct: record.mint_acct.clone(),
        owner_acct: record.owner_acct.clone(),
        amount: new_amount,
        created_at: SystemTime::now(),
    };

    connection
        .interact(move |conn| {
            diesel::insert_into(token_acct_balances::table)
                .values(new_balance)
                .execute(conn)
                .expect("Error inserting into token_acct_balances");

            diesel::update(token_accts::table.filter(token_acct.eq(record.token_acct)))
                .set(amount.eq(new_amount))
                .execute(conn)
                .expect("Error updating token_accts");
        })
        .await?;
    Ok(())
}
