use std::env;
use std::sync::Arc;

use chrono::Utc;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::{update, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use futures::StreamExt;
use solana_account_decoder::{UiAccount, UiAccountData};
use solana_client::{nonblocking::pubsub_client::PubsubClient, rpc_config::RpcAccountInfoConfig};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::sync::Mutex;
use tokio::task;

use crate::adapters;
use crate::entities::token_accts::token_accts::{self};
use crate::entities::token_accts::{TokenAcct, TokenAcctStatus};
use crate::entities::transactions::transactions::{self, tx_sig};
use crate::entities::transactions::Transaction;
use crate::services::balances;
use crate::services::transactions::handle_token_acct_balance_tx;
use diesel::OptionalExtension;

use bigdecimal::BigDecimal;

pub async fn new_handler(
    pub_sub_client: Arc<PubsubClient>,
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    token_acct_pubkey: Pubkey,
    token_acct_record: TokenAcct,
) {
    
    let rpc_endpoint = env::var("RPC_ENDPOINT_HTTP").expect("RPC_ENDPOINT_HTTP must be set");
    if let Err(e) = check_and_update_initial_balance(
        rpc_endpoint,
        Arc::clone(&conn_manager),
        &token_acct_pubkey,
        &token_acct_record,
    )
    .await
    {
      eprintln!("Error during initial balance check: {:?}", e);
    }

    let timeout_flag = Arc::new(Mutex::new(true));

    let account_subscribe_res = pub_sub_client
        .account_subscribe(
            &token_acct_pubkey,
            Some(RpcAccountInfoConfig {
                encoding: Some(solana_account_decoder::UiAccountEncoding::JsonParsed),
                data_slice: None,
                commitment: Some(CommitmentConfig::confirmed()),
                min_context_slot: None,
            }),
        )
        .await;

    if account_subscribe_res.is_err() {
        eprintln!(
            "error when subscribing to account, {:?}",
            account_subscribe_res.err().unwrap()
        );
        return;
    }

    println!(
        "successfully subscribed to token acct: {}",
        token_acct_pubkey.to_string()
    );

    let (mut subscription, _) = account_subscribe_res.ok().unwrap();

    let conn_manager_clone_sub = Arc::clone(&conn_manager);
    while let Some(val) = subscription.next().await {
        let mut timeout_flag_val = timeout_flag.lock().unwrap();
        *timeout_flag_val = false;
        let ui_account: UiAccount = val.value;
        let context = val.context;
        println!("account subscribe context: {:?}", context);
        match ui_account.data {
            UiAccountData::Binary(data, encoding) => {
                println!("Binary data: {:?}, Encoding: {:?}", data, encoding);
            }
            UiAccountData::Json(data) => {
                println!("account subscribe notification: {:?}", data);
                let record_clone = token_acct_record.clone();
                let token_acct_clone = record_clone.token_acct.clone();
                let conn_clone_for_task = Arc::clone(&conn_manager_clone_sub);
                task::spawn(async move {
                    let token_acct_update_res = balances::handle_token_acct_change(
                        conn_clone_for_task,
                        record_clone,
                        data,
                        context,
                    )
                    .await;
                    match token_acct_update_res {
                        Ok(_) => {
                            println!("successfully updated token balance: {:?}", token_acct_clone)
                        }
                        Err(e) => println!("error kind: {:?}", e),
                    }
                });
            }
            UiAccountData::LegacyBinary(data) => {
                println!("Parsed LegacyBinary data: {:?}", data);
            }
        }
    }
    println!(
        "end of rpc account subscriber scope: {}",
        token_acct_pubkey.to_string()
    );

    // enabling this seems questionable now... since when this function fires we will not reset things to
    // update_token_acct_with_status(
    //     token_acct_pubkey.to_string(),
    //     TokenAcctStatus::Enabled,
    //     conn_manager,
    // )
    // .await;
}

async fn check_and_update_initial_balance(
    rpc_endpoint: String,
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    token_acct_pubkey: &Pubkey,
    token_acct_record: &TokenAcct,
) -> Result<(), Box<dyn std::error::Error>> {

    let rpc_client = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(
        rpc_endpoint,
    ));
    let token_account = adapters::rpc::get_token_account_by_address(
        Arc::clone(&rpc_client),
        token_acct_pubkey.to_string(),
    )
    .await?;
    let balance = BigDecimal::from(token_account.amount);

    if !token_acct_record.amount.eq(&balance) {
        let latest_tx: Vec<
            solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature,
        > = rpc_client
            .get_signatures_for_address(token_acct_pubkey)
            .await?
            .into_iter()
            .filter(|tx| tx.err.is_none())
            .collect();

        if let Some(latest_tx_info) = latest_tx.first() {
            let transaction_sig = latest_tx_info.signature.clone();
            let transaction_sig_2 = latest_tx_info.signature.clone();
            let transaction_exists: Option<Transaction> = conn_manager
                .interact(move |db: &mut PgConnection| {
                    transactions::table
                        .filter(tx_sig.eq(transaction_sig.clone()))
                        .first::<Transaction>(db)
                        .optional()
                })
                .await??;

            let slot = BigDecimal::from(latest_tx_info.slot);
            let mint_acct = token_account.mint.to_string();
            let owner_acct = token_account.owner.to_string();

            let transaction_sig_option = if transaction_exists.is_some() {
                Some(transaction_sig_2)
            } else {
                None
            };

            handle_token_acct_balance_tx(
                conn_manager,
                token_acct_pubkey.to_string(),
                balance,
                transaction_sig_option,
                slot,
                mint_acct,
                owner_acct,
            )
            .await?;
        }
    }
    

    Ok(())
}

// TODO this should return a result and be handled appropriately..
async fn update_token_acct_with_status(
    token_acct: String,
    status: TokenAcctStatus,
    conn_manager: Arc<Object<Manager<PgConnection>>>,
) {
    let token_acct_query = token_acct.clone();
    let status_for_set = status.clone();
    let res = conn_manager
        .interact(move |db| {
            update(
                token_accts::table.filter(token_accts::token_acct.eq(token_acct_query.to_string())),
            )
            .set((
                token_accts::dsl::status.eq(status_for_set),
                token_accts::dsl::updated_at.eq(Utc::now()),
            ))
            .get_result::<TokenAcct>(db)
        })
        .await;

    match res {
        Ok(Ok(_)) => println!(
            "updated token acct to {:?} status: {}",
            status,
            token_acct.to_string()
        ),
        Err(e) => eprintln!(
            "error updating token acct [{}] to {:?} status: {}",
            token_acct, status, e
        ),
        Ok(Err(e)) => eprintln!(
            "error updating token acct [{}] to {:?} status: {}",
            token_acct, status, e
        ),
    }
}
