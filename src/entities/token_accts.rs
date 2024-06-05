use std::time::SystemTime;

table! {
    token_accts (token_acct) {
        token_acct -> Varchar,
        mint_acct -> Varchar,
        owner_acct -> Varchar,
        amount -> Float,
        updated_at -> Nullable<Timestamp>,
    }
}

#[derive(Queryable)]
pub struct TokenAcct {
    pub token_acct: String,
    pub mint_acct: String,
    pub owner_acct: String,
    pub amount: f32,
    pub updated_at: Option<SystemTime>,
}
