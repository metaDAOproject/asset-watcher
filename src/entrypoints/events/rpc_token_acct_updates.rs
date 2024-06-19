use std::env;
use std::sync::{Arc, MutexGuard};

use diesel::{update, Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use futures::StreamExt;
use solana_account_decoder::{UiAccount, UiAccountData};
use solana_client::{nonblocking::pubsub_client::PubsubClient, rpc_config::RpcAccountInfoConfig};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::sync::Mutex;
use tokio::task;

use crate::entities::token_accts::token_accts::{self};
use crate::entities::token_accts::{TokenAcct, TokenAcctStatus};
use crate::services::balances;

pub async fn new_handler(
    pub_sub_client: Arc<PubsubClient>,
    db: &mut PgConnection,
    token_acct_pubkey: Pubkey,
    token_acct_record: TokenAcct,
) {
    println!("subscribing to acct: {}", token_acct_pubkey.to_string());
    // TODO: use the watching status to persist what accounts are being watched, if it's already being watched, then it should be
    let timeout_flag = Arc::new(Mutex::new(true));
    let timeout_flag_arc = Arc::clone(&timeout_flag);
    // TODO this cannot be calling expect, we need to not panic here
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

    // update token to watching status
    update_token_acct_with_status(token_acct_pubkey.to_string(), TokenAcctStatus::Watching, db);

    let (mut subscription, unsubscribe) = account_subscribe_res.ok().unwrap();

    // spawn that timeout task
    task::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::new(60, 0)).await;
            let mut timeout_flag_val: MutexGuard<bool> = timeout_flag_arc.lock().unwrap();
            if *timeout_flag_val {
                break;
            }

            *timeout_flag_val = true;
        }
        println!(
            "timed out. unsubscribing from account: {}",
            token_acct_pubkey.to_string()
        );
        // we have to construct a fresh connection here because of the new thread
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let mut new_conn =
            PgConnection::establish(&database_url).expect("could not establish connection");
        update_token_acct_with_status(
            token_acct_pubkey.to_string(),
            TokenAcctStatus::Enabled,
            &mut new_conn,
        );
        unsubscribe();
    });

    while let Some(val) = subscription.next().await {
        // reset timeout flag
        let mut timeout_flag_val = timeout_flag.lock().unwrap();
        *timeout_flag_val = false;
        let ui_account: UiAccount = val.value;
        let context = val.context;
        println!("account subscribe context: {:?}", context);
        // tODO handle the result
        match ui_account.data {
            UiAccountData::Binary(data, encoding) => {
                println!("Binary data: {:?}, Encoding: {:?}", data, encoding);
                // Process binary data here
            }
            UiAccountData::Json(data) => {
                println!("account subscribe notification: {:?}", data);
                let record_clone = token_acct_record.clone();
                let token_acct_clone = record_clone.token_acct.clone();
                let token_acct_update_res =
                    balances::handle_token_acct_change(db, record_clone, data, context);
                match token_acct_update_res {
                    Ok(_) => {
                        println!("successfully updated token balance: {:?}", token_acct_clone)
                    }
                    Err(e) => println!("error kind: {:?}", e),
                }
                // Process JSON data here
            }
            UiAccountData::LegacyBinary(data) => {
                println!("Parsed LegacyBinary data: {:?}", data);
                // Process parsed JSON data here
            }
        }
    }
    println!(
        "end of rpc account subscriber scope: {}",
        token_acct_pubkey.to_string()
    );

    //update token back to enabled status
    update_token_acct_with_status(token_acct_pubkey.to_string(), TokenAcctStatus::Enabled, db);
}

fn update_token_acct_with_status(
    token_acct: String,
    status: TokenAcctStatus,
    db: &mut PgConnection,
) {
    //update token back to enabled status
    let res = update(token_accts::table.filter(token_accts::token_acct.eq(token_acct.to_string())))
        .set(token_accts::dsl::status.eq(status.clone()))
        .get_result::<TokenAcct>(db);

    match res {
        Ok(_) => println!(
            "updated token acct to {:?} status: {}",
            status,
            token_acct.to_string()
        ),
        Err(e) => eprintln!(
            "error updating token acct [{}] to watching status: {}",
            token_acct, e
        ),
    }
}
