use crate::entities::deposits::user_deposits;
use crate::entities::{deposits::UserDeposit, transactions::Instruction};
use bigdecimal::BigDecimal;
use chrono::Utc;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::{PgConnection, RunQueryDsl};
use std::sync::Arc;

pub async fn handle_deposit(
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    tx_sig: String,
    authority_account: String,
    mint_instruction: &Instruction,
    mint_acct: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let amount: i64 = mint_instruction
        .args
        .iter() // Create an iterator over the arguments
        .find(|arg| arg.name == "amount") // Find the argument with the name "amount"
        .and_then(|arg| arg.data.parse().ok()) // Parse the data
        .unwrap_or(0); // Handle the case where the argument is not found

    let deposit = UserDeposit::new(authority_account, BigDecimal::from(amount), mint_acct, tx_sig);

    conn_manager
        .interact(move |db| {
            diesel::insert_into(user_deposits::table)
                .values(&deposit)
                .execute(db)
        })
        .await??;

    Ok(())
}
