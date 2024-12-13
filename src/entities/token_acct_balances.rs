use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};

table! {
    token_acct_balances (token_acct, mint_acct, amount, created_at) {
        token_acct -> Varchar,
        mint_acct -> Varchar,
        owner_acct -> Varchar,
        amount -> Numeric,
        created_at -> Timestamptz,
        slot -> Numeric,
        tx_sig -> Nullable<Varchar>,
        delta -> Numeric,
    }
}

#[derive(Queryable, Clone, Insertable, Selectable)]
#[diesel(table_name = token_acct_balances)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TokenAcctBalances {
    pub token_acct: String,
    pub mint_acct: String,
    pub owner_acct: String,
    pub amount: BigDecimal,
    pub created_at: DateTime<Utc>,
    pub slot: BigDecimal,
    pub tx_sig: Option<String>,
    pub delta: BigDecimal,
}
