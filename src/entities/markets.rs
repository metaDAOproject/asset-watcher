use chrono::{DateTime, Utc};

table! {
    use diesel::sql_types::*;

    markets (market_acct) {
        market_acct -> Varchar,
        market_type -> Varchar,
        create_tx_sig -> Varchar,
        proposal_acct -> Nullable<Varchar>,
        base_mint_acct -> Varchar,
        quote_mint_acct -> Varchar,
        created_at -> Timestamptz,
    }
}

#[derive(Queryable, Insertable, Debug, Clone, Selectable)]
#[diesel(table_name = markets)]
pub struct Market {
    pub market_acct: String,
    pub market_type: String,
    pub create_tx_sig: String,
    pub proposal_acct: Option<String>,
    pub base_mint_acct: String,
    pub quote_mint_acct: String,
    pub created_at: DateTime<Utc>,
}
