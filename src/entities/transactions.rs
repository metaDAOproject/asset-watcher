use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, Output, ToSql},
    sql_types::Text,
};
use std::time::SystemTime;

table! {
    transactions (tx_sig) {
        tx_sig -> Varchar,
        slot -> BigInt,
        block_time -> Timestamp,
        failed -> Bool,
        payload -> Text,
        serializer_logic_version -> SmallInt,
        main_ix_type -> Varchar,
    }
}

#[derive(Debug, Clone, Copy, AsExpression, FromSqlRow)]
#[sql_type = "Text"]
pub enum InstructionType {
    VaultMintConditionalTokens,
    AmmSwap,
    AmmDeposit,
    AmmWithdraw,
    OpenbookPlaceOrder,
    OpenbookCancelOrder,
    AutocratInitializeProposal,
    AutocratFinalizeProposal,
}

impl<DB> ToSql<Text, DB> for InstructionType
where
    DB: Backend,
    str: ToSql<Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match self {
            InstructionType::VaultMintConditionalTokens => {
                "vault_mint_conditional_tokens".to_sql(out)
            }
            InstructionType::AmmSwap => "amm_swap".to_sql(out),
            InstructionType::AmmDeposit => "amm_deposit".to_sql(out),
            InstructionType::AmmWithdraw => "amm_withdraw".to_sql(out),
            InstructionType::OpenbookPlaceOrder => "openbook_place_order".to_sql(out),
            InstructionType::OpenbookCancelOrder => "openbook_cancel_order".to_sql(out),
            InstructionType::AutocratInitializeProposal => {
                "autocrat_initialize_proposal".to_sql(out)
            }
            InstructionType::AutocratFinalizeProposal => "autocrat_finalize_proposal".to_sql(out),
        }
    }
}

impl FromSql<Text, Pg> for InstructionType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"vault_mint_conditional_tokens" => Ok(InstructionType::VaultMintConditionalTokens),
            b"amm_swap" => Ok(InstructionType::AmmSwap),
            b"amm_deposit" => Ok(InstructionType::AmmDeposit),
            b"amm_withdraw" => Ok(InstructionType::AmmWithdraw),
            b"openbook_place_order" => Ok(InstructionType::OpenbookPlaceOrder),
            b"openbook_cancel_order" => Ok(InstructionType::OpenbookCancelOrder),
            b"autocrat_initialize_proposal" => Ok(InstructionType::AutocratInitializeProposal),
            b"autocrat_finalize_proposal" => Ok(InstructionType::AutocratFinalizeProposal),
            x => Err(format!("Unrecognized variant {:?}", x).into()),
        }
    }
}

#[derive(Queryable, Insertable)]
#[table_name = "transactions"]
pub struct Transaction {
    pub tx_sig: String,
    pub slot: i64,
    pub block_time: SystemTime,
    pub failed: bool,
    pub payload: String,
    pub serializer_logic_version: i16,
    pub main_ix_type: InstructionType,
}
