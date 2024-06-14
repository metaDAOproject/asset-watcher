use std::{env, sync::Arc};

pub async fn get_pubsub_client(
) -> Result<Arc<solana_client::nonblocking::pubsub_client::PubsubClient>, Box<dyn std::error::Error>>
{
    let rpc_endpoint_ws = env::var("RPC_ENDPOINT_WSS").expect("RPC_ENDPOINT_WSS must be set");
    let pub_sub_client =
        solana_client::nonblocking::pubsub_client::PubsubClient::new(&rpc_endpoint_ws).await?;
    Ok(Arc::new(pub_sub_client))
}
