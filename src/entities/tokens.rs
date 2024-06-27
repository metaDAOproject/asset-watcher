use chrono::{DateTime, Utc};

table! {
    tokens (mint_acct) {
        mint_acct -> Varchar,
        name -> Varchar,
        symbol -> Varchar,
        supply -> BigInt,
        decimals -> SmallInt,
        updated_at -> Timestamptz,
        image_url -> Nullable<Varchar>,
    }
}

#[derive(Queryable, Clone, Selectable)]
#[diesel(table_name = tokens)]
pub struct Token {
    pub mint_acct: String,
    pub name: String,
    pub symbol: String,
    pub supply: i64,
    pub decimals: i16,
    pub updated_at: DateTime<Utc>,
    pub image_url: Option<String>,
}
