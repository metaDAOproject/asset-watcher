use std::sync::Arc;

use crate::entities::conditional_vaults::conditional_vaults::dsl::*;
use crate::entities::conditional_vaults::ConditionalVault;
use crate::entities::transactions::Instruction;
use crate::entities::transactions::Payload;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::PgConnection;

use super::balances;

pub async fn handle_merge_conditional_tokens_tx(
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    transaction_payload: Payload,
    transaction_sig: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mint_instruction = find_mint_instruction(&transaction_payload)?;
    let authority_account = find_authority_account(&mint_instruction)?;
    let vault_account = find_vault_account(&mint_instruction)?;
    let conditional_vault =
        get_conditional_vault(Arc::clone(&conn_manager), &vault_account).await?;

    let relevant_accounts =
        get_relevant_accounts_from_mint_and_vault(&mint_instruction, conditional_vault);

    for (token_account, mint_acct_value) in relevant_accounts {
        balances::handle_token_acct_in_tx(
            Arc::clone(&conn_manager),
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

fn find_mint_instruction(
    transaction_payload: &Payload,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    transaction_payload
        .instructions
        .iter()
        .find(|instruction| instruction.name == "mergeConditionalTokensForUnderlyingTokens")
        .cloned()
        .ok_or_else(|| "mergeConditionalTokensForUnderlyingTokens instruction not found".into())
}

fn find_authority_account(
    mint_instruction: &Instruction,
) -> Result<String, Box<dyn std::error::Error>> {
    mint_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "authority")
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| {
            "Authority account not found in mergeConditionalTokensForUnderlyingTokens instruction"
                .into()
        })
}

fn find_vault_account(
    mint_instruction: &Instruction,
) -> Result<String, Box<dyn std::error::Error>> {
    mint_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "vault")
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| {
            "Vault account not found in mergeConditionalTokensForUnderlyingTokens instruction"
                .into()
        })
}

async fn get_conditional_vault(
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    vault_account: &str,
) -> Result<ConditionalVault, Box<dyn std::error::Error>> {
    let vault_acct_clone = vault_account.to_string();
    let vault = conn_manager
        .interact(move |connection| {
            conditional_vaults
                .filter(cond_vault_acct.eq(vault_acct_clone))
                .first(connection)
        })
        .await??;

    Ok(vault)
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
