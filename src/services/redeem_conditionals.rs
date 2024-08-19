use std::sync::Arc;

use super::balances;
use super::transactions;
use crate::entities::conditional_vaults::conditional_vaults::dsl::*;
use crate::entities::conditional_vaults::ConditionalVault;
use crate::entities::transactions::Instruction;
use crate::entities::transactions::Payload;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::PgConnection;

pub async fn handle_redeem_conditional_tokens_tx(
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    transaction_payload: &Payload,
    transaction_sig: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mint_instruction = transactions::find_instruction(
        &transaction_payload,
        "redeemConditionalTokensForUnderlyingTokens",
    )?;
    let authority_account = transactions::find_authority_account(&mint_instruction)?;
    let vault_account = transactions::find_vault_account(&mint_instruction)?;
    let conditional_vault =
        transactions::get_conditional_vault(Arc::clone(&conn_manager), &vault_account).await?;

    let relevant_accounts = transactions::get_relevant_accounts_from_mint_and_vault(
        &mint_instruction,
        &conditional_vault,
    );

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
