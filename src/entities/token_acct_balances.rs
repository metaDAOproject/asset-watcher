use std::time::SystemTime;

table! {
    token_acct_balances (token_acct) {
        token_acct -> Varchar,
        mint_acct -> Varchar,
        owner_acct -> Varchar,
        amount -> Double,
        created_at -> Timestamp,
    }
}

#[derive(Queryable)]
pub struct TokenAcctBalances {
    pub token_acct: String,
    pub mint_acct: String,
    pub owner_acct: String,
    pub amount: f64,
    pub created_at: SystemTime,
}

#[derive(Insertable)]
#[diesel(table_name = token_acct_balances)]
pub struct TokenAcctBalancesRecord {
    pub token_acct: String,
    pub mint_acct: String,
    pub owner_acct: String,
    pub amount: f64,
    pub created_at: SystemTime,
}
