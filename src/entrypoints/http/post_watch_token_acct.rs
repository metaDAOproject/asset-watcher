use std::env;
use std::str::FromStr;
use std::sync::Arc;

use crate::entities::token_accts::token_accts;
use crate::entities::token_accts::token_accts::dsl::*;
use crate::entities::token_accts::TokenAcct;
use crate::entities::token_accts::TokenAcctStatus;
use crate::entities::token_accts::WatchTokenBalancePayload;
use crate::entities::token_accts::WatchTokenBalanceResponse;
use chrono::Utc;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::update;
use diesel::PgConnection;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::program_pack::Pack;
use spl_token::state::Account;
use warp::{reject::Reject, Reply};

#[derive(Debug)]
struct ParseError;
impl Reject for ParseError {}

pub async fn handler(
    reply_with_status: warp::reply::WithStatus<&'static str>,
    message: WatchTokenBalancePayload,
    conn_manager: Arc<Object<Manager<PgConnection>>>,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::Rejection> {
    let response = reply_with_status.into_response();
    if !response.status().is_success() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&WatchTokenBalanceResponse {
                message: "unsuccessful response status".to_string(),
            }),
            response.status(),
        ));
    }

    let token_acct_pubkey = message.token_acct.clone();
    let token_acct_for_query = token_acct_pubkey.clone();

    let token_acct_res: Result<Result<Option<TokenAcct>, _>, deadpool_diesel::InteractError> =
        conn_manager
            .interact(move |db| {
                token_accts
                    .filter(token_accts::dsl::token_acct.eq(&token_acct_for_query))
                    .first::<TokenAcct>(db)
                    .optional()
            })
            .await;

    let token_acct_for_update = token_acct_pubkey.clone();
    let token_acct_for_insert = token_acct_pubkey.clone();
    match token_acct_res {
        Ok(Ok(Some(token_acct_record))) => {
            // if already watching, we need to switch back to enabled and then back to make sure account subscribe reinits
            if token_acct_record.status == TokenAcctStatus::Watching {
                let enabled_update_res = conn_manager
                    .interact(move |db| {
                        update_token_acct_with_status(
                            token_acct_for_update,
                            TokenAcctStatus::Enabled,
                            db,
                        )
                    })
                    .await;

                match enabled_update_res {
                    Ok(_) => (),
                    Err(e) => {
                        return Ok(warp::reply::with_status(
                            warp::reply::json(&WatchTokenBalanceResponse {
                                message: format!(
                                    "error updating token acct [{}] to {:?} status: {}",
                                    token_acct_pubkey.to_string(),
                                    status,
                                    e
                                ),
                            }),
                            warp::http::StatusCode::BAD_REQUEST,
                        ));
                    }
                }
            }
        }
        Err(e) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&WatchTokenBalanceResponse {
                    message: format!("error interacting with postgres pool: {:?}", e),
                }),
                response.status(),
            ));
        }
        Ok(Err(e)) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&WatchTokenBalanceResponse {
                    message: format!("error fetching token_acct to update: {:?}", e),
                }),
                response.status(),
            ));
        }
        Ok(Ok(None)) => {
            // add token acct creation here
            let rpc_endpoint =
                env::var("RPC_ENDPOINT_HTTP").expect("RPC_ENDPOINT_HTTP must be set");
            let rpc_client = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(
                rpc_endpoint,
            ));
            let token_acct_pubkey_str = token_acct_for_insert.clone();
            let token_acct_pubkey =
                match solana_sdk::pubkey::Pubkey::from_str(&token_acct_pubkey_str) {
                    Ok(pubkey) => pubkey,
                    Err(_) => {
                        return Ok(warp::reply::with_status(
                            warp::reply::json(&WatchTokenBalanceResponse {
                                message: "Invalid token account public key".to_string(),
                            }),
                            warp::http::StatusCode::BAD_REQUEST,
                        ));
                    }
                };

            let account_res = rpc_client
                .get_account_with_commitment(&token_acct_pubkey, CommitmentConfig::confirmed())
                .await;

            let account_data = match account_res {
                Ok(data) => data,
                Err(_) => {
                    return Ok(warp::reply::with_status(
                        warp::reply::json(&WatchTokenBalanceResponse {
                            message: "Failed to fetch account data".to_string(),
                        }),
                        warp::http::StatusCode::BAD_REQUEST,
                    ));
                }
            };

            if let Some(account) = account_data.value {
                let token_account: Account = match Account::unpack(&account.data) {
                    Ok(account) => account,
                    Err(_) => {
                        return Ok(warp::reply::with_status(
                            warp::reply::json(&WatchTokenBalanceResponse {
                                message: "Failed to unpack token account data".to_string(),
                            }),
                            warp::http::StatusCode::BAD_REQUEST,
                        ));
                    }
                };

                let new_token_acct = TokenAcct {
                    token_acct: token_acct_for_insert.clone(),
                    owner_acct: token_account.owner.to_string(),
                    amount: 0,
                    status: TokenAcctStatus::Watching,
                    mint_acct: token_account.mint.to_string(),
                    updated_at: Some(Utc::now()),
                };

                let new_token_acct_clone = new_token_acct.clone();

                let insert_res = conn_manager
                    .interact(move |db| {
                        diesel::insert_into(token_accts::table)
                            .values(&new_token_acct_clone)
                            .execute(db)
                    })
                    .await;
                return match insert_res {
                    Ok(_) => Ok(warp::reply::with_status(
                        warp::reply::json(&WatchTokenBalanceResponse {
                            message: format!("inserted new token acct: {}", token_acct_for_insert),
                        }),
                        warp::http::StatusCode::OK,
                    )),
                    _ => Ok(warp::reply::with_status(
                        warp::reply::json(&WatchTokenBalanceResponse {
                            message: format!("could not insert new token_acct to watch"),
                        }),
                        response.status(),
                    )),
                };
            } else {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&WatchTokenBalanceResponse {
                        message: format!(
                            "could not find token_acct spl data for new token balance insert"
                        ),
                    }),
                    response.status(),
                ));
            }
        }
    }

    let token_acct_for_watching_update = token_acct_pubkey.clone();
    let res = conn_manager
        .interact(move |db| {
            update_token_acct_with_status(
                token_acct_for_watching_update,
                TokenAcctStatus::Watching,
                db,
            )
        })
        .await;

    match res {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&WatchTokenBalanceResponse {
                message: format!(
                    "updated token acct to {:?} status: {}",
                    status,
                    token_acct_pubkey.to_string()
                ),
            }),
            warp::http::StatusCode::OK,
        )),
        Err(e) => Ok(warp::reply::with_status(
            warp::reply::json(&WatchTokenBalanceResponse {
                message: format!(
                    "error updating token acct [{}] to {:?} status: {}",
                    token_acct_pubkey.to_string(),
                    status,
                    e
                ),
            }),
            warp::http::StatusCode::BAD_REQUEST,
        )),
    }
}

fn update_token_acct_with_status(
    token_acct_pubkey: String,
    token_acct_status: TokenAcctStatus,
    db: &mut PgConnection,
) -> Result<TokenAcct, diesel::result::Error> {
    update(token_accts::table.filter(token_accts::token_acct.eq(token_acct_pubkey.to_string())))
        .set((
            token_accts::dsl::status.eq(token_acct_status.clone()),
            token_accts::dsl::updated_at.eq(Utc::now()),
        ))
        .get_result::<TokenAcct>(db)
}
