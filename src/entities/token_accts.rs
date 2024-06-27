use chrono::{DateTime, Utc};
use diesel::pg::Pg;
use diesel::serialize::IsNull;
use diesel::{
    deserialize::{self, FromSql},
    pg::PgValue,
    serialize::{self, Output, ToSql},
    AsExpression,
};
use serde::{Deserialize, Serialize};
use std::io::Write;

table! {
    token_accts (token_acct) {
        token_acct -> Varchar,
        mint_acct -> Varchar,
        owner_acct -> Varchar,
        amount -> BigInt,
        updated_at -> Nullable<Timestamptz>,
        status -> crate::entities::token_accts::TokenAcctStatusType,
    }
}

#[derive(Queryable, Clone, Insertable, Selectable)]
pub struct TokenAcct {
    pub token_acct: String,
    pub mint_acct: String,
    pub owner_acct: String,
    pub amount: i64,
    pub updated_at: Option<DateTime<Utc>>,
    pub status: TokenAcctStatus,
}

#[derive(SqlType, QueryId)]
#[diesel(postgres_type(name = "token_acct_status"))]
pub struct TokenAcctStatusType;

#[derive(
    Debug, PartialEq, FromSqlRow, AsExpression, Eq, Clone, Hash, serde::Deserialize, Serialize,
)]
#[diesel(sql_type = TokenAcctStatusType)]
#[serde(rename_all = "lowercase")]
pub enum TokenAcctStatus {
    Watching,
    Enabled,
    Disabled,
}

impl ToString for TokenAcctStatus {
    fn to_string(&self) -> String {
        match self {
            TokenAcctStatus::Watching => "watching".to_string(),
            TokenAcctStatus::Enabled => "enabled".to_string(),
            TokenAcctStatus::Disabled => "disabled".to_string(),
        }
    }
}

impl ToSql<TokenAcctStatusType, Pg> for TokenAcctStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            TokenAcctStatus::Watching => out.write_all(b"watching")?,
            TokenAcctStatus::Enabled => out.write_all(b"enabled")?,
            TokenAcctStatus::Disabled => out.write_all(b"disabled")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<TokenAcctStatusType, Pg> for TokenAcctStatus {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"enabled" => Ok(TokenAcctStatus::Enabled),
            b"disabled" => Ok(TokenAcctStatus::Disabled),
            b"watching" => Ok(TokenAcctStatus::Watching),
            x => Err(format!("Unrecognized variant {:?}", x).into()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenAcctsInsertChannelPayload {
    pub token_acct: String,
}

impl TokenAcctsInsertChannelPayload {
    pub fn parse_payload(
        json_str: &str,
    ) -> Result<TokenAcctsInsertChannelPayload, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

// todo setup serialization
#[derive(Serialize, Deserialize, Debug)]
pub struct TokenAcctsStatusUpdateChannelPayload {
    pub status: TokenAcctStatus,
    pub token_acct: String,
}

impl TokenAcctsStatusUpdateChannelPayload {
    pub fn parse_payload(
        json_str: &str,
    ) -> Result<TokenAcctsStatusUpdateChannelPayload, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchTokenBalancePayload {
    pub token_acct: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WatchTokenBalanceResponse {
    pub message: String,
}
