use std::time::SystemTime;

use crate::entities::conditional_vaults::conditional_vaults::dsl::*;
use crate::entities::conditional_vaults::ConditionalVault;
use crate::entities::token_acct_balances::token_acct_balances::dsl::*;
use crate::entities::token_acct_balances::TokenAcctBalancesRecord;
use crate::entities::token_accts::token_accts;
use crate::entities::token_accts::token_accts::dsl::*;
use crate::entities::token_accts::TokenAcct;
use crate::entities::token_accts::TokenAcctStatus;
use crate::entities::transactions::Payload;
use diesel::prelude::*;
use diesel::PgConnection;

pub fn handle_mint_tx(
    connection: &mut PgConnection,
    transaction_payload: Payload,
) -> Result<(), Box<dyn std::error::Error>> {
    // Find the "mintConditionalTokens" instruction
    let mint_instruction = transaction_payload
        .instructions
        .iter()
        .find(|instruction| instruction.name == "mintConditionalTokens")
        .ok_or("mintConditionalTokens instruction not found")?;

    // Find the "authority" account pubkey in the instruction
    let authority_account = mint_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "authority")
        .ok_or("Authority account not found in mintConditionalTokens instruction")?
        .pubkey
        .clone();

    // Find the "vault" account pubkey in the instruction
    let vault_account = mint_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "vault")
        .ok_or("Vault account not found in mintConditionalTokens instruction")?
        .pubkey
        .clone();

    let conditional_vault: ConditionalVault = conditional_vaults
        .filter(cond_vault_acct.eq(vault_account.clone()))
        .first(connection)?;

    // Collect the necessary "user" accounts to insert into token_accts
    let relevant_accounts: Vec<(&str, &str)> = mint_instruction
        .accounts_with_data
        .iter()
        .filter_map(|account| match account.name.as_str() {
            "userConditionalOnFinalizeTokenAccount" => Some((
                account.pubkey.as_str(),
                conditional_vault.cond_finalize_token_mint_acct.as_str(),
            )),
            "userConditionalOnRevertTokenAccount" => Some((
                account.pubkey.as_str(),
                conditional_vault.cond_revert_token_mint_acct.as_str(),
            )),
            "userUnderlyingTokenAccount" => Some((
                account.pubkey.as_str(),
                conditional_vault.underlying_mint_acct.as_str(),
            )),
            _ => None,
        })
        .collect();

    for (token_account, mint_acct_value) in relevant_accounts {
        // Check if the token account already exists
        let exists = token_accts
            .filter(token_accts::dsl::token_acct.eq(token_account))
            .count()
            .get_result::<i64>(connection)?
            > 0;

        // If the token account already exists, skip the insert
        if exists {
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

        let new_token_acct = TokenAcct {
            token_acct: token_account.to_string(),
            owner_acct: authority_account.clone(),
            amount: account_balance,
            status: TokenAcctStatus::Watching,
            mint_acct: mint_acct_value.to_string(),
            updated_at: Some(SystemTime::now()),
        };
        let new_token_acct_balance = TokenAcctBalancesRecord {
            token_acct: token_account.to_string(),
            mint_acct: mint_acct_value.to_string(),
            owner_acct: authority_account.clone(),
            amount: account_balance,
            created_at: SystemTime::now(),
        };
        diesel::insert_into(token_accts)
            .values(&new_token_acct)
            .execute(connection)?;
        diesel::insert_into(token_acct_balances)
            .values(&new_token_acct_balance)
            .execute(connection)?;
    }

    Ok(())
}
