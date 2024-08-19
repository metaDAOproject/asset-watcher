use std::sync::Arc;

use chrono::Utc;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::PgConnection;

use crate::entities::conditional_vaults::conditional_vaults::dsl::*;
use crate::entities::token_acct_balances::token_acct_balances;
use crate::entities::token_acct_balances::TokenAcctBalances;
use crate::entities::token_accts::token_accts;
use crate::entities::conditional_vaults::ConditionalVault;
use crate::entities::transactions::Instruction;
use crate::entities::transactions::Payload;
use crate::entities::markets::Market;
use crate::entities::markets::markets;
use crate::entities::markets::markets::market_acct;
// use crate::entrypoints::events;

/**
 * Handles updating our DB for a tx that affects a token acct balance.
 * Will update both token_accts and token_acct_balances table with the new balance amount
 */
pub async fn handle_token_acct_balance_tx(
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    token_acct: String,
    new_balance: i64,
    transaction_sig: Option<String>,
    slot: i64,
    mint_acct: String,
    owner_acct: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Query the most recent value for the given token_acct to calculate the delta
    let token_acct_clone_1 = token_acct.clone();
    let previous_balance: Option<_> = conn_manager
        .interact(move |db| {
            token_acct_balances::table
                .filter(token_acct_balances::token_acct.eq(token_acct_clone_1))
                .order(token_acct_balances::slot.desc())
                .select(token_acct_balances::amount)
                .first::<i64>(db)
                .optional()
        })
        .await??;

    let delta = match previous_balance {
        Some(prev_amount) => new_balance - prev_amount,
        None => new_balance,
    };

    let token_acct_clone_2 = token_acct.clone();
    let existing_balance_res = conn_manager
        .interact(move |db| {
            token_acct_balances::table
                .filter(
                    token_acct_balances::slot
                        .eq(slot)
                        .and(token_acct_balances::token_acct.eq(token_acct_clone_2)),
                )
                .first::<TokenAcctBalances>(db)
        })
        .await?;

    let maybe_balance = existing_balance_res.ok();

    if let Some(balance) = maybe_balance {
        if balance.tx_sig.is_none() {
            let token_acct_clone_3 = token_acct.clone();
            let tx_sig_clone = transaction_sig.clone();
            conn_manager
                .interact(move |db| {
                    diesel::update(
                        token_acct_balances::table.filter(
                            token_acct_balances::token_acct
                                .eq(token_acct_clone_3)
                                .and(token_acct_balances::slot.eq(slot)),
                        ),
                    )
                    .set(token_acct_balances::tx_sig.eq(tx_sig_clone))
                    .execute(db)
                })
                .await??;
        }
        // already has the correct tx_sig, no need to update anything
    } else {
        let new_token_acct_balance = TokenAcctBalances {
            token_acct: token_acct.clone(),
            mint_acct: mint_acct,
            owner_acct: owner_acct,
            amount: new_balance,
            delta,
            slot: slot,
            tx_sig: transaction_sig,
            created_at: Utc::now(),
        };

        conn_manager
            .interact(move |db| {
                diesel::insert_into(token_acct_balances::table)
                    .values(&new_token_acct_balance)
                    .execute(db)
            })
            .await??;
    }

    // Update the token_accts table with the new balance in the amount column
    let token_acct_clone_4 = token_acct.clone();
    conn_manager
        .interact(move |db| {
            diesel::update(
                token_accts::table.filter(token_accts::token_acct.eq(token_acct_clone_4)),
            )
            .set((
                token_accts::amount.eq(new_balance),
                token_accts::dsl::updated_at.eq(Utc::now()),
            ))
            .execute(db)
        })
        .await??;

    Ok(())
}


pub fn find_instruction(
    transaction_payload: &Payload,
    instruction_name: &str,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    transaction_payload
        .instructions
        .iter()
        .find(|instruction| instruction.name == instruction_name.to_string())
        .cloned()
        .ok_or_else(|| "Instruction not found".into())
}

pub fn find_authority_account(
    mint_instruction: &Instruction,
) -> Result<String, Box<dyn std::error::Error>> {
    mint_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "authority")
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| "Authority account not found in mintConditionalTokens instruction".into())
}

pub fn find_vault_account(
    mint_instruction: &Instruction,
) -> Result<String, Box<dyn std::error::Error>> {
    mint_instruction
        .accounts_with_data
        .iter()
        .find(|account| account.name == "vault")
        .map(|account| account.pubkey.clone())
        .ok_or_else(|| "Vault account not found in mintConditionalTokens instruction".into())
}

pub async fn get_conditional_vault(
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

pub fn get_relevant_accounts_from_mint_and_vault(
    mint_instruction: &Instruction,
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

pub fn get_relevant_accounts_from_ix_and_mints(
    mint_instruction: &Instruction,
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

pub async fn find_base_and_quote_mint(
    amm_acct: String,
    conn_manager: Arc<Object<Manager<PgConnection>>>,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let amm_market: Market = conn_manager
        .interact(|connection| {
            markets::table
                .filter(market_acct.eq(amm_acct))
                .first(connection)
        })
        .await??;

    Ok((amm_market.base_mint_acct, amm_market.quote_mint_acct))
}