use diesel::pg::Pg;
use diesel::serialize::IsNull;
use diesel::sql_types::Text;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    pg::PgValue,
    serialize::{self, Output, ToSql},
    AsExpression,
};
use std::io::Write;
use std::time::SystemTime;

table! {
    token_accts (token_acct) {
        token_acct -> Varchar,
        mint_acct -> Varchar,
        owner_acct -> Varchar,
        amount -> Double,
        updated_at -> Nullable<Timestamp>,
        status -> crate::entities::token_accts::TokenAcctStatusType,
    }
}

#[derive(Queryable, Clone)]
pub struct TokenAcct {
    pub token_acct: String,
    pub mint_acct: String,
    pub owner_acct: String,
    pub amount: f64,
    pub updated_at: Option<SystemTime>,
    pub status: TokenAcctStatus,
}

#[derive(SqlType, QueryId)]
#[diesel(postgres_type(name = "TokenAcctStatus", schema = "TokenAccts"))]
pub struct TokenAcctStatusType;

#[derive(Debug, PartialEq, FromSqlRow, AsExpression, Eq, Clone, Hash, serde::Deserialize)]
#[diesel(sql_type = TokenAcctStatusType)]
pub enum TokenAcctStatus {
    Watching,
    Enabled,
    Disabled,
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
        let value = String::from_sql(bytes);
        match bytes.as_bytes() {
            b"enabled" => Ok(TokenAcctStatus::Enabled),
            b"disabled" => Ok(TokenAcctStatus::Disabled),
            b"watching" => Ok(TokenAcctStatus::Watching),
            x => Err(format!("Unrecognized variant {:?}", x).into()),
        }
    }
}
