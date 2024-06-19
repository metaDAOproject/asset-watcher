use std::{env, sync::Arc};

use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::PgConnection;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use tokio::sync::Mutex;
use warp::http::Method;
use warp::Filter;

use crate::{entities::token_accts::WatchTokenBalancePayload, services::auth::AuthClient};

use super::post_watch_token_acct;

pub async fn listen_and_serve(
    db: Arc<Object<Manager<PgConnection>>>,
    rpc_pub_sub: Arc<PubsubClient>,
) {
    let port = match env::var("PORT")
        .unwrap_or("8080".to_string())
        .parse::<u16>()
    {
        Ok(port) => port,
        Err(_) => 8080,
    };
    let auth_service_url = env::var("AUTH_SERVICE_URL").expect("AUTH_SERVICE_URL must be set");
    let auth_client = Arc::new(Mutex::new(AuthClient::new(&auth_service_url)));

    let auth_filter = warp::any()
        .and(warp::header::<String>("authorization"))
        .and(with_auth_client(auth_client.clone()))
        .and_then(validate_token);

    let watch_balance_route = warp::post()
        .and(warp::path("watch-token-balance"))
        .and(auth_filter)
        .and(watch_token_json_body())
        .and(with_db(db))
        .and(with_rpc_pub_sub(rpc_pub_sub))
        .and_then(post_watch_token_acct::handler);

    let cors =
        warp::cors().allow_methods(&[Method::POST, Method::DELETE, Method::OPTIONS, Method::GET]);

    let routes = watch_balance_route.with(cors);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await
}

fn watch_token_json_body(
) -> impl Filter<Extract = (WatchTokenBalancePayload,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

fn with_auth_client(
    auth_client: Arc<Mutex<AuthClient>>,
) -> impl Filter<Extract = (Arc<Mutex<AuthClient>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || auth_client.clone())
}
fn with_db(
    db: Arc<Object<Manager<PgConnection>>>,
) -> impl Filter<Extract = (Arc<Object<Manager<PgConnection>>>,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || db.clone())
}
fn with_rpc_pub_sub(
    rpc_pub_sub: Arc<PubsubClient>,
) -> impl Filter<Extract = (Arc<PubsubClient>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || rpc_pub_sub.clone())
}

async fn validate_token(
    token: String,
    auth_client: Arc<Mutex<AuthClient>>,
) -> Result<warp::reply::WithStatus<&'static str>, warp::Rejection> {
    let auth_client = auth_client.lock().await;

    // Check if the token starts with "Bearer "
    if !token.starts_with("Bearer ") {
        return Ok(warp::reply::with_status(
            "Invalid authorization header",
            warp::http::StatusCode::BAD_REQUEST,
        ));
    }

    // Extract the actual token by removing the "Bearer " prefix
    let token_trimmed = token.trim_start_matches("Bearer ");

    match auth_client.post_session(&token_trimmed).await {
        Ok(response) => {
            println!("Valid token! Session ID: {}", response.session_id);
            Ok(warp::reply::with_status(
                "Token is valid",
                warp::http::StatusCode::OK,
            ))
        }
        Err(err) => {
            println!("Invalid token: {}", err);
            Ok(warp::reply::with_status(
                "Token is invalid",
                warp::http::StatusCode::UNAUTHORIZED,
            ))
        }
    }
}
