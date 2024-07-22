use std::{env, str::FromStr, sync::Arc};

use solana_sdk::{commitment_config::CommitmentConfig, program_pack::Pack, pubkey::Pubkey};

pub async fn get_pubsub_client(
) -> Result<Arc<solana_client::nonblocking::pubsub_client::PubsubClient>, Box<dyn std::error::Error>>
{
    let rpc_endpoint_ws = env::var("RPC_ENDPOINT_WSS").expect("RPC_ENDPOINT_WSS must be set");
    let pub_sub_client =
        solana_client::nonblocking::pubsub_client::PubsubClient::new(&rpc_endpoint_ws).await?;
    Ok(Arc::new(pub_sub_client))
}

pub async fn get_token_account_by_address(
    rpc_client: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
    token_acct_address: String,
) -> Result<spl_token::state::Account, Box<dyn std::error::Error>> {
    let token_acct_pubkey = Pubkey::from_str(&token_acct_address)?;
    let account_data = rpc_client
        .get_account_with_commitment(&token_acct_pubkey, CommitmentConfig::confirmed())
        .await?;

    if let Some(account) = account_data.value {
        let token_account: spl_token::state::Account =
            spl_token::state::Account::unpack(&account.data)?;
        Ok(token_account)
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "could not find token acct",
        )))
    }
}
