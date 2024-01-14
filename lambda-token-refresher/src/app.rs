/// App related functions
use aws_sdk_ssm::{types::{Parameter, ParameterType}, Client as AWSClient};
use hyper::{body::to_bytes, Body, Client, Method, Request};
use hyper_rustls::HttpsConnectorBuilder;
use lambda_runtime::Error;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, to_string};

/// Token struct holding each token type
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
}

/// Overarching secret response
#[derive(Serialize, Deserialize, Debug)]
pub struct Secret {
    pub secret: SecretItem,
}

/// The items in the secret payload
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SecretItem {
    #[serde(rename = "_id")]
    pub id: Option<String>,
    pub version: u64,
    pub workspace: String,
    #[serde(rename = "type")]
    pub secret_type: String,
    pub secret_key: String,
    pub secret_value: String,
    pub secret_comment: String,
}

/// Refresh the tokens for the app using the refresh token.
pub async fn refresh_token(
    endpoint: &str,
    client_id: &str,
    refresh_token: &str,
    scope: &str,
) -> Result<Token, Error> {
    let input = [
        ("client_id", Some(client_id)),
        ("scope", Some(scope)),
        ("refresh_token", Some(refresh_token)),
        ("grant_type", Some("refresh_token")),
    ];
    let data = serde_urlencoded::to_string(input).expect("Error serializing form");

    let https = HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_only()
        .enable_http1()
        .build();

    let client: Client<_, Body> = Client::builder().build(https);
    let req = Request::builder()
        .method(Method::POST)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .uri(endpoint)
        .body(Body::from(data))
        .expect("request builder");

    let res = client.request(req).await?;
    tracing::info!("Token refresh status code: {:?}", res.status());
    let body_bytes = to_bytes(res.into_body()).await?;
    let body = String::from_utf8(body_bytes.to_vec())?;
    let body: Token = from_str(&body)?;
    Ok(body)
}

/// Update a secret at source.
pub async fn update_secret(
    endpoint: &str,
    secret_key: &str,
    new_value: &str,
    secret_path: &str,
    environment: &str,
    workspace: &str,
    bearer: &str,
) -> Result<Secret, Box<dyn std::error::Error>> {
    let url = format!("{}{}", endpoint, secret_key);
    let payload = to_string(&json!({
        "environment": environment,
        "secretValue": new_value,
        "workspaceId": workspace,
        "secretPath": secret_path
    }))
    .unwrap();

    let con = HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_only()
        .enable_http1()
        .build();
    let client: Client<_, Body> = Client::builder().build(con);

    let req = Request::builder()
        .method(Method::PATCH)
        .header("Authorization", format!("Bearer {}", bearer))
        .header("Content-Type", "application/json")
        .uri(url)
        .body(Body::from(payload))
        .expect("Failed to build request");

    let res = client.request(req).await?;
    tracing::info!("Secret update status code: {:?}", res.status());

    let body_bytes = to_bytes(res.into_body()).await?;
    let body = String::from_utf8(body_bytes.to_vec())?;
    let body: Secret = from_str(&body)?;
    Ok(body)
}

/// Retrieve a parameter from the SSM parameter store
pub async fn get_ssm_parameter(client: &AWSClient, param: &str) -> Result<Parameter, Error> {
    let res = client
        .get_parameter()
        .name(param)
        .with_decryption(true)
        .send()
        .await?
        .parameter;

    match res {
        Some(p) => Ok(p),
        None => Err(format!("Unable to locate parameter {}", param).into()),
    }
}

/// Retrieve a parameter from the SSM parameter store
///
/// temporary
pub async fn set_ssm_parameter(client: &AWSClient, param: &str, value: &str) -> Result<(), Error> {
    let res = client
        .put_parameter()
        .overwrite(true)
        .r#type(ParameterType::SecureString)
        .name(format!("/onedrive-manager/{}", param))
        .value(value)
        .send()
        .await;

    tracing::info!("Secret {}, update status: {:?}", param, res);

    Ok(())
}
