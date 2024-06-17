use chrono::{DateTime, Utc};
use diesel::pg::{Pg, PgValue};
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, Output, ToSql},
    sql_types::Text,
};
use serde::{Deserialize, Serialize};

table! {
    transactions (tx_sig) {
        tx_sig -> Varchar,
        slot -> BigInt,
        block_time -> Timestamptz,
        failed -> Bool,
        payload -> Text,
        serializer_logic_version -> SmallInt,
        main_ix_type -> Nullable<Varchar>,
    }
}

#[derive(Queryable, Clone, Selectable)]
#[diesel(table_name = transactions)]
pub struct Transaction {
    pub tx_sig: String,
    pub slot: i64,
    pub block_time: DateTime<Utc>,
    pub failed: bool,
    pub payload: String,
    pub serializer_logic_version: i16,
    pub main_ix_type: Option<InstructionType>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    pub block_time: i64,
    pub slot: i64,
    pub recent_blockhash: String,
    pub compute_units_consumed: String,
    pub fee: String,
    pub signatures: Vec<String>,
    pub version: String,
    pub log_messages: Vec<String>,
    pub accounts: Vec<Account>,
    pub instructions: Vec<Instruction>,
}

impl Payload {
    pub fn parse_payload(json_str: &str) -> Result<Payload, serde_json::Error> {
        serde_json::from_str(json_str)
    }
    pub fn get_main_ix_type(&self) -> Option<InstructionType> {
        for ix in &self.instructions {
            match ix.name.as_str() {
                "swap" => return Some(InstructionType::AmmSwap),
                "addLiquidity" => return Some(InstructionType::AmmDeposit),
                "removeLiquidity" => return Some(InstructionType::AmmWithdraw),
                "placeOrder" => return Some(InstructionType::OpenbookPlaceOrder),
                "cancelOrder" => return Some(InstructionType::OpenbookCancelOrder),
                "mintConditionalTokens" => {
                    return Some(InstructionType::VaultMintConditionalTokens)
                }
                "initializeProposal" => return Some(InstructionType::AutocratInitializeProposal),
                "finalizeProposal" => return Some(InstructionType::AutocratFinalizeProposal),
                _ => continue,
            }
        }
        None
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub name: String,
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writeable: bool,
    pub pre_balance: Option<String>,
    pub post_balance: Option<String>,
    pub pre_token_balance: Option<TokenBalance>,
    pub post_token_balance: Option<TokenBalance>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenBalance {
    pub mint: String,
    pub owner: String,
    pub amount: String,
    pub decimals: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    pub name: String,
    pub stack_height: u8,
    pub program_id_index: u8,
    pub data: String,
    pub accounts: Vec<u8>,
    pub accounts_with_data: Vec<AccountWithData>,
    pub args: Vec<Arg>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountWithData {
    pub name: String,
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writeable: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Arg {
    pub name: String,
    #[serde(rename = "type")]
    arg_type: String,
    data: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionsInsertChannelPayload {
    pub tx_sig: String,
}

impl TransactionsInsertChannelPayload {
    pub fn parse_payload(
        json_str: &str,
    ) -> Result<TransactionsInsertChannelPayload, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}
