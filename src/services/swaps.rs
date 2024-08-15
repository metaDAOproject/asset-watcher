use std::io;
use std::sync::Arc;

use crate::entities::transactions::Instruction;
use crate::entities::transactions::Payload;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::PgConnection;

use super::{balances,transactions};

pub async fn handle_swap_tx(
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    transaction_payload: &Payload,
    transaction_sig: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let swap_instruction = find_swap_instruction(&transaction_payload)?;
    let authority_account = transactions::find_authority_account(&swap_instruction)?;
    let amm_acct = swap_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "amm")
        .map(|account| account.pubkey.clone());

    let amm_acct_str = match amm_acct {
        Some(acct) => acct,
        None => String::default().to_string(),
    };

    if amm_acct_str == "" {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "amm_acct not found",
        )));
    }

    let (base_mint, quote_mint) =
        transactions::find_base_and_quote_mint(amm_acct_str, Arc::clone(&conn_manager)).await?;

    let relevant_accounts =
        transactions::get_relevant_accounts_from_ix_and_mints(&swap_instruction, base_mint, quote_mint);

    for (token_account, mint_acct_value) in relevant_accounts {
        balances::handle_token_acct_in_tx(
            Arc::clone(&conn_manager),
            transaction_payload,
            transaction_sig.clone(),
            &mint_acct_value,
            token_account,
            &authority_account,
        )
        .await?
    }

    Ok(())
}

fn find_swap_instruction(
    transaction_payload: &Payload,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    transaction_payload
        .instructions
        .iter()
        .find(|instruction| instruction.name == "swap")
        .cloned()
        .ok_or_else(|| "swap instruction not found".into())
}