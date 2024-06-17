use std::sync::Arc;

use crate::entities::conditional_vaults::conditional_vaults::dsl::*;
use crate::entities::conditional_vaults::ConditionalVault;
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

pub async fn handle_mint_tx(
    connection: &mut PgConnection,
    pub_sub_client: Arc<PubsubClient>,
    transaction_payload: Payload,
    transaction_sig: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mint_instruction = find_mint_instruction(&transaction_payload)?;
    let authority_account = find_authority_account(&mint_instruction)?;
    let vault_account = find_vault_account(&mint_instruction)?;
    let conditional_vault = get_conditional_vault(connection, &vault_account)?;

    let relevant_accounts =
        get_relevant_accounts_from_mint_and_vault(&mint_instruction, conditional_vault);

    for (token_account, mint_acct_value) in relevant_accounts {
        // Check if the token record exists
        let token_record_exists = tokens::tokens::table
            .filter(tokens::tokens::dsl::mint_acct.eq(mint_acct_value.to_string()))
            .count()
            .get_result::<i64>(connection)?
            > 0;

        // If the token record does not exist, log a warning and skip
        if !token_record_exists {
            println!("Token record not found for account: {}", token_account);
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

fn find_mint_instruction(
    transaction_payload: &Payload,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    transaction_payload
        .instructions
        .iter()
        .find(|instruction| instruction.name == "mintConditionalTokens")
        .cloned()
        .ok_or_else(|| "mintConditionalTokens instruction not found".into())
}

fn find_authority_account(
    mint_instruction: &Instruction,
) -> Result<String, Box<dyn std::error::Error>> {
    mint_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "authority")
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| "Authority account not found in mintConditionalTokens instruction".into())
}

fn find_vault_account(
    mint_instruction: &Instruction,
) -> Result<String, Box<dyn std::error::Error>> {
    mint_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "vault")
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| "Vault account not found in mintConditionalTokens instruction".into())
}

fn get_conditional_vault(
    connection: &mut PgConnection,
    vault_account: &str,
) -> Result<ConditionalVault, Box<dyn std::error::Error>> {
    conditional_vaults
        .filter(cond_vault_acct.eq(vault_account))
        .first(connection)
        .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)
}

fn get_relevant_accounts_from_mint_and_vault(
    mint_instruction: &crate::entities::transactions::Instruction,
    conditional_vault: ConditionalVault,
) -> Vec<(&str, String)> {
    // Collect the necessary "user" accounts to insert into token_accts

    let relevant_accounts: Vec<(&str, String)> = mint_instruction
        .accounts_with_data
        .iter()
        .filter_map(|account| {
            let vault_clone = conditional_vault.clone();
            match account.name.as_str() {
                "userConditionalOnFinalizeTokenAccount" => Some((
                    account.pubkey.as_str(),
                    vault_clone.cond_finalize_token_mint_acct,
                )),
                "userConditionalOnRevertTokenAccount" => Some((
                    account.pubkey.as_str(),
                    vault_clone.cond_revert_token_mint_acct,
                )),
                "userUnderlyingTokenAccount" => {
                    Some((account.pubkey.as_str(), vault_clone.underlying_mint_acct))
                }
                _ => None,
            }
        })
        .collect();
    relevant_accounts
}
