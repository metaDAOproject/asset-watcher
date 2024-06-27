use chrono::{DateTime, Utc};

table! {
    token_acct_balances (token_acct, mint_acct, amount, created_at) {
        token_acct -> Varchar,
        mint_acct -> Varchar,
        owner_acct -> Varchar,
        amount -> BigInt,
        created_at -> Timestamptz,
        slot -> BigInt,
        tx_sig -> Nullable<Varchar>,
        delta -> BigInt,
    }
}

#[derive(Queryable, Clone, Insertable, Selectable)]
#[diesel(table_name = token_acct_balances)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TokenAcctBalances {
    pub token_acct: String,
    pub mint_acct: String,
    pub owner_acct: String,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
    pub slot: i64,
    pub tx_sig: Option<String>,
    pub delta: i64,
}
