use diesel::prelude::*;

table! {
    conditional_vaults (cond_vault_acct) {
        cond_vault_acct -> Varchar,
        status -> Nullable<Varchar>,
        settlement_authority -> Varchar,
        underlying_mint_acct -> Varchar,
        underlying_token_acct -> Varchar,
        nonce -> Nullable<Varchar>,
        cond_finalize_token_mint_acct -> Varchar,
        cond_revert_token_mint_acct -> Varchar,
    }
}

#[derive(Queryable)]
pub struct ConditionalVault {
    pub cond_vault_acct: String,
    pub status: Option<String>,
    pub settlement_authority: String,
    pub underlying_mint_acct: String,
    pub underlying_token_acct: String,
    pub nonce: Option<String>,
    pub cond_finalize_token_mint_acct: String,
    pub cond_revert_token_mint_acct: String,
}

#[derive(Insertable)]
#[diesel(table_name = conditional_vaults)]
pub struct NewConditionalVault<'a> {
    pub cond_vault_acct: &'a str,
    pub status: Option<&'a str>,
    pub settlement_authority: &'a str,
    pub underlying_mint_acct: &'a str,
    pub underlying_token_acct: &'a str,
    pub nonce: Option<&'a str>,
    pub cond_finalize_token_mint_acct: &'a str,
    pub cond_revert_token_mint_acct: &'a str,
}
