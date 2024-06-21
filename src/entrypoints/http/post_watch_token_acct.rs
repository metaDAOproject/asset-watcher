use std::sync::Arc;

use crate::entities::token_accts::token_accts;
use crate::entities::token_accts::token_accts::dsl::*;
use crate::entities::token_accts::TokenAcct;
use crate::entities::token_accts::TokenAcctStatus;
use crate::entities::token_accts::WatchTokenBalancePayload;
use crate::entities::token_accts::WatchTokenBalanceResponse;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::update;
use diesel::PgConnection;
use warp::{reject::Reject, Reply};

#[derive(Debug)]
struct ParseError;
impl Reject for ParseError {}

pub async fn handler(
    reply_with_status: warp::reply::WithStatus<&'static str>,
    message: WatchTokenBalancePayload,
    conn_manager: Arc<Object<Manager<PgConnection>>>,
) -> Result<impl warp::Reply, warp::Rejection> {
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
            return Ok(warp::reply::with_status(
                warp::reply::json(&WatchTokenBalanceResponse {
                    message: format!("could not find token_acct to update"),
                }),
                response.status(),
            ));
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
        .set(token_accts::dsl::status.eq(token_acct_status.clone()))
        .get_result::<TokenAcct>(db)
}
