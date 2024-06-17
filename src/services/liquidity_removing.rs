use std::io;
use std::sync::Arc;

use crate::entities::markets::markets;
use crate::entities::markets::markets::market_acct;
use crate::entities::markets::Market;
use crate::entities::token_accts::token_accts;
use crate::entities::token_accts::token_accts::dsl::*;
use crate::entities::token_accts::TokenAcct;
use crate::entities::token_accts::TokenAcctStatus;
use crate::entities::tokens;
use crate::entities::transactions::Instruction;
use crate::entities::transactions::Payload;
use chrono::Utc;
use diesel::prelude::*;
use diesel::PgConnection;
use solana_client::nonblocking::pubsub_client::PubsubClient;

use super::transactions;

pub async fn handle_lp_withdrawal_tx(
    connection: &mut PgConnection,
    pub_sub_client: Arc<PubsubClient>,
    transaction_payload: Payload,
    transaction_sig: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let lp_withdrawal_instruction = find_lp_withdrawal_instruction(&transaction_payload)?;
    let authority_account = find_authority_account(&lp_withdrawal_instruction)?;
    let (lp_ata, lp_mint) = find_lp_mint_and_ata_account(&lp_withdrawal_instruction)?;
    let mut lp_account_vec = vec![(lp_ata.as_str(), lp_mint)];
    let amm_acct = lp_withdrawal_instruction
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
            "This is an error message",
        )));
    }

    let (base_mint, quote_mint) = find_base_and_quote_mint(amm_acct_str, connection)?;

    let mut relevant_accounts =
        get_relevant_accounts_from_ix_and_mints(&lp_withdrawal_instruction, base_mint, quote_mint);
    relevant_accounts.append(&mut lp_account_vec);

    for (token_account, mint_acct_value) in relevant_accounts {
        // Check if the token record exists
        let token_record_exists = tokens::tokens::table
            .filter(tokens::tokens::dsl::mint_acct.eq(mint_acct_value.to_string()))
            .count()
            .get_result::<i64>(connection)?
            > 0;

        // If the token record does not exist, log a warning and skip
        if !token_record_exists {
            println!(
                "Token table record not found for account: {}",
                token_account
            );
            continue;
        }

        // Find the matching account in the root accounts array and extract postBalance
        let account_with_balance = transaction_payload
            .accounts
            .iter()
            .find(|acc| acc.pubkey == token_account)
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
        let token_acct_record: Vec<TokenAcct> = token_accts
            .filter(token_accts::dsl::token_acct.eq(token_account))
            .load::<TokenAcct>(connection)?;

        // If the token account does not exist, insert it!
        if Vec::len(&token_acct_record) > 0 {
            let new_token_acct = TokenAcct {
                token_acct: token_account.to_string(),
                owner_acct: authority_account.clone(),
                amount: account_balance,
                status: TokenAcctStatus::Watching,
                mint_acct: mint_acct_value.to_string(),
                updated_at: Some(Utc::now()),
            };

            diesel::insert_into(token_accts)
                .values(&new_token_acct)
                .execute(connection)?;
        }

        transactions::handle_token_acct_balance_tx(
            connection,
            Arc::clone(&pub_sub_client),
            token_account.to_string(),
            account_balance,
            transaction_sig.clone(),
            transaction_payload.slot,
            token_acct_record[0].status == TokenAcctStatus::Watching,
        )
        .await?;
    }

    Ok(())
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

fn find_authority_account(
    lp_withdrawal_instruction: &Instruction,
) -> Result<String, Box<dyn std::error::Error>> {
    lp_withdrawal_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "user")
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| "Authority account not found in addLiquidity instruction".into())
}
fn find_lp_mint_and_ata_account(
    lp_withdrawal_instruction: &Instruction,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mint_res: Result<String, &str> = lp_withdrawal_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "lpMint")
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| "lpMint account not found in addLiquidity instruction");
    let ata_res: Result<String, &str> = lp_withdrawal_instruction
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
    lp_withdrawal_instruction: &crate::entities::transactions::Instruction,
    base_mint: String,
    quote_mint: String,
) -> Vec<(&str, String)> {
    // Collect the necessary "user" accounts to insert into token_accts

    let relevant_accounts: Vec<(&str, String)> = lp_withdrawal_instruction
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
