use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;

table! {
    user_deposits (user_acct) {
        user_acct -> Varchar,
        token_amount -> Numeric,
        mint_acct -> Varchar,
        tx_sig -> Varchar,
        created_at -> Timestamptz,
    }
}

#[derive(Queryable, Clone, Insertable, Selectable)]
#[diesel(table_name = user_deposits)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserDeposit {
    pub user_acct: String,
    pub token_amount: BigDecimal,
    pub mint_acct: String,
    pub tx_sig: String,
    pub created_at: DateTime<Utc>,
}

impl UserDeposit {
    pub fn new(user_acct: String, token_amount: BigDecimal, mint_acct: String, tx_sig: String) -> Self {
        UserDeposit {
            user_acct,
            token_amount,
            mint_acct,
            tx_sig,
            created_at: Utc::now(),
        }
    }
}