use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthSessionResponse {
    pub session_id: String,
    pub was_logged_in: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthMessageResponse {
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthErrorResponse {
    error: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthPostRequest {
    pub_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthPutRequest {
    id: String,
    signature: String,
    pub_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthGetRequest {
    pubkey: String,
}

pub struct AuthClient {
    client: Client,
    base_url: String,
}

impl AuthClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn post_session(&self, pub_key: &str) -> Result<AuthSessionResponse, Box<dyn Error>> {
        let url = format!("{}/auth", self.base_url);
        let req_body = AuthPostRequest {
            pub_key: pub_key.to_string(),
        };
        let resp = self.client.post(&url).json(&req_body).send().await?;
        println!("{}", resp.status());
        if resp.status().is_success() {
            let session_response = resp.json::<AuthSessionResponse>().await?;
            Ok(session_response)
        } else {
            let error_response = resp.json::<AuthErrorResponse>().await?;
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                error_response.error,
            )))
        }
    }

    pub async fn put_session(
        &self,
        id: &str,
        signature: &str,
        pub_key: &str,
    ) -> Result<AuthMessageResponse, Box<dyn Error>> {
        let url = format!("{}/auth", self.base_url);
        let req_body = AuthPutRequest {
            id: id.to_string(),
            signature: signature.to_string(),
            pub_key: pub_key.to_string(),
        };
        let resp = self.client.put(&url).json(&req_body).send().await?;
        if resp.status().is_success() {
            let message_response = resp.json::<AuthMessageResponse>().await?;
            Ok(message_response)
        } else {
            let error_response = resp.json::<AuthErrorResponse>().await?;
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                error_response.error,
            )))
        }
    }

    pub async fn get_session(&self, pubkey: &str) -> Result<AuthMessageResponse, Box<dyn Error>> {
        let url = format!("{}/auth", self.base_url);
        let req_body = AuthGetRequest {
            pubkey: pubkey.to_string(),
        };
        let resp = self.client.get(&url).json(&req_body).send().await?;
        if resp.status().is_success() {
            let message_response = resp.json::<AuthMessageResponse>().await?;
            Ok(message_response)
        } else {
            let error_response = resp.json::<AuthErrorResponse>().await?;
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                error_response.error,
            )))
        }
    }
}
