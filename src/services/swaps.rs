use std::io;

use crate::entities::markets::markets;
use crate::entities::markets::markets::market_acct;
use crate::entities::markets::Market;
use crate::entities::transactions::Instruction;
use crate::entities::transactions::Payload;
use diesel::prelude::*;
use diesel::PgConnection;

use super::balances;

pub async fn handle_swap_tx(
    connection: &mut PgConnection,
    transaction_payload: Payload,
    transaction_sig: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let swap_instruction = find_swap_instruction(&transaction_payload)?;
    let authority_account = find_authority_account(&swap_instruction)?;
    let amm_acct = swap_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "amm")
        .map(|account| account.pubkey.clone());

    let amm_acct_str = match amm_acct {
        Some(acct) => acct,
        None => "".to_string(),
    };

    if amm_acct_str == "" {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "amm_acct not found",
        )));
    }

    let (base_mint, quote_mint) = find_base_and_quote_mint(amm_acct_str, connection)?;

    let relevant_accounts =
        get_relevant_accounts_from_ix_and_mints(&swap_instruction, base_mint, quote_mint);

    for (token_account, mint_acct_value) in relevant_accounts {
        balances::handle_token_acct_in_tx(
            connection,
            transaction_payload.clone(),
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

fn find_authority_account(
    mint_instruction: &Instruction,
) -> Result<String, Box<dyn std::error::Error>> {
    mint_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "user")
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| "Authority account not found in swap instruction".into())
}

fn find_base_and_quote_mint(
    amm_acct: String,
    connection: &mut PgConnection,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let amm_market: Market = markets::table
        .filter(market_acct.eq(amm_acct))
        .first(connection)?;

    Ok((amm_market.base_mint_acct, amm_market.quote_mint_acct))
}

fn get_relevant_accounts_from_ix_and_mints(
    mint_instruction: &crate::entities::transactions::Instruction,
    base_mint: String,
    quote_mint: String,
) -> Vec<(&str, String)> {
    // Collect the necessary "user" accounts to insert into token_accts

    let relevant_accounts: Vec<(&str, String)> = mint_instruction
        .accounts_with_data
        .iter()
        .filter_map(|account| match account.name.as_str() {
            "userBaseAccount" => Some((account.pubkey.as_str(), base_mint.clone())),
            "userQuoteAccount" => Some((account.pubkey.as_str(), quote_mint.clone())),
            _ => None,
        })
        .collect();
    relevant_accounts
}
