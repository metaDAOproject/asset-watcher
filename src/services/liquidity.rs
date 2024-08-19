use std::io;
use std::sync::Arc;

use crate::entities::transactions::Instruction;
use crate::entities::transactions::Payload;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::PgConnection;

use super::{balances,transactions};

pub async fn handle_lp_deposit_tx(
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    transaction_payload: &Payload,
    transaction_sig: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let lp_deposit_instruction = find_lp_deposit_instruction(&transaction_payload)?;
    let authority_account = transactions::find_authority_account(&lp_deposit_instruction)?;
    let (lp_ata, lp_mint) = find_lp_mint_and_ata_account(&lp_deposit_instruction)?;
    let mut lp_account_vec = vec![(lp_ata.as_str(), lp_mint)];
    let amm_acct = lp_deposit_instruction
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
            "no amm_acct_str",
        )));
    }

    let (base_mint, quote_mint) =
        transactions::find_base_and_quote_mint(amm_acct_str, Arc::clone(&conn_manager)).await?;

    let mut relevant_accounts =
        transactions::get_relevant_accounts_from_ix_and_mints(&lp_deposit_instruction, base_mint, quote_mint);
    relevant_accounts.append(&mut lp_account_vec);

    for (token_account, mint_acct_value) in &relevant_accounts {
        balances::handle_token_acct_in_tx(
            Arc::clone(&conn_manager),
            transaction_payload,
            transaction_sig.clone(),
            mint_acct_value,
            token_account,
            &authority_account,
        )
        .await?
    }

    Ok(())
}


pub async fn handle_lp_withdrawal_tx(
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    transaction_payload: &Payload,
    transaction_sig: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let lp_withdrawal_instruction = find_lp_withdrawal_instruction(&transaction_payload)?;
    let authority_account = transactions::find_authority_account(&lp_withdrawal_instruction)?;
    let (lp_ata, lp_mint) = find_lp_mint_and_ata_account(&lp_withdrawal_instruction)?;
    let mut lp_account_vec = vec![(lp_ata.as_str(), lp_mint)];
    let amm_acct = lp_withdrawal_instruction
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
            "This is an error message",
        )));
    }

    let (base_mint, quote_mint) =
        transactions::find_base_and_quote_mint(amm_acct_str, Arc::clone(&conn_manager)).await?;

    let mut relevant_accounts =
        transactions::get_relevant_accounts_from_ix_and_mints(&lp_withdrawal_instruction, base_mint, quote_mint);
    relevant_accounts.append(&mut lp_account_vec);

    for (token_account, mint_acct_value) in &relevant_accounts {
        balances::handle_token_acct_in_tx(
            Arc::clone(&conn_manager),
            transaction_payload,
            transaction_sig.clone(),
            mint_acct_value,
            token_account,
            &authority_account,
        )
        .await?
    }

    Ok(())
}

fn find_lp_deposit_instruction(
    transaction_payload: &Payload,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    transaction_payload
        .instructions
        .iter()
        .find(|instruction| instruction.name == "addLiquidity")
        .cloned()
        .ok_or_else(|| "addLiquidity instruction not found".into())
}

fn find_lp_mint_and_ata_account(
    lp_deposit_instruction: &Instruction,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mint_res: Result<String, &str> = lp_deposit_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "lpMint")
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| "lpMint account not found in addLiquidity instruction");
    let ata_res: Result<String, &str> = lp_deposit_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "userLpAccount")
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| "lpMint account not found in addLiquidity instruction");

    match (mint_res, ata_res) {
        (Ok(mint), Ok(ata)) => Ok((ata, mint)),
        _ => Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "could not find lp accounts",
        ))),
    }
}

fn find_lp_withdrawal_instruction(
    transaction_payload: &Payload,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    transaction_payload
        .instructions
        .iter()
        .find(|instruction| instruction.name == "addLiquidity")
        .cloned()
        .ok_or_else(|| "addLiquidity instruction not found".into())
}