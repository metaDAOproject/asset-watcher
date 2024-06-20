use serde::{Deserialize, Serialize};

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
    pub error: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthPostRequest {
    pub pub_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthPutRequest {
    pub id: String,
    pub signature: String,
    pub pub_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthGetRequest {
    pub pubkey: String,
}
