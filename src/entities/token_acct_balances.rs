use std::time::SystemTime;

table! {
    token_accts (token_acct) {
        token_acct -> Varchar,
        mint_acct -> Varchar,
        owner_acct -> Varchar,
        amount -> Int8,
        created_at -> Timestamp,
    }
}

#[derive(Queryable)]
pub struct TokenAcctBalances {
    pub token_acct: String,
    pub mint_acct: String,
    pub owner_acct: String,
    pub amount: i64,
    pub created_at: SystemTime,
}
