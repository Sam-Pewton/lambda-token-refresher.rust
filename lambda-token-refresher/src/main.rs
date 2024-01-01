//! Lambda function to keep bearer tokens warm.
//!
//! This is written for a personal project.
use aws_config::BehaviorVersion;
use aws_sdk_ssm::Client as AWSClient;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
mod app;

/// Function response
///
/// Currently doesn't get used by anything.
#[derive(Serialize)]
pub struct Response {
    req_id: String,
    successful: bool,
}

/// Function input received from AWS EventBridge
#[derive(Deserialize, Debug, Clone)]
pub struct EventBridgePayload {
    cid_path: String,
    scope_path: String,
    r_tok_path: String,
    update_paths: Vec<String>,
    ssm_retrieval_paths: Vec<String>,
    secrets_endpoint: String,
    app_endpoint: String,
    secrets_path: String,
}

/// Main function handler for the lambda function
pub async fn function_handler(event: LambdaEvent<EventBridgePayload>) -> Result<Response, Error> {
    // get parameters from AWS SSM
    let config = aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
    let client = Arc::new(AWSClient::new(&config));

    let mut handles = vec![];
    let ssm_map = Arc::new(Mutex::new(HashMap::new()));

    for key in event.payload.ssm_retrieval_paths {
        let this_client = Arc::clone(&client);
        let this_map = ssm_map.clone();
        let handle = tokio::spawn(async move {
            let param = app::get_ssm_parameter(&this_client, &key)
                .await
                .expect(&format!("Problem retrieving parameter `{}` from SSM", key));
            let mut shared = this_map.lock().await;
            shared.insert(key, param.value.expect(""));
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Error joining thread handles");
    }

    // get new token and save to infisical
    let updated: app::Token;
    {
        let ssm_lock = ssm_map.lock().await;
        updated = app::refresh_token(
            &event.payload.app_endpoint,
            ssm_lock
                .get(&event.payload.cid_path)
                .expect("Error accessing client_id"),
            ssm_lock
                .get(&event.payload.r_tok_path)
                .expect("Error accessing refresh token"),
            ssm_lock
                .get(&event.payload.scope_path)
                .expect("Error accessing scope"),
        )
        .await?;
    }

    // update the appropriate tokens with newly retrieved values
    let mut handles = vec![];
    for key in event.payload.update_paths {
        let this_map = ssm_map.clone();
        let updated_cp: HashMap<String, String> = from_value(to_value(updated.clone())?)?;
        let s_endpoint = event.payload.secrets_endpoint.clone();
        let s_path = event.payload.secrets_path.clone();
        let handle = tokio::spawn(async move {
            let shared_map = this_map.lock().await;
            let _ = app::update_secret(
                &s_endpoint,
                &key,
                &updated_cp.get(&key.to_lowercase()).expect("Error getting key"),
                &s_path,
                &shared_map.get("secrets-environment").expect("Error getting env"),
                &shared_map.get("secrets-workspace").expect("Error getting workspace"),
                &shared_map.get("secrets-rw").expect("Error getting token"),
            )
            .await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Error joining thread handles");
    }

    // Prepare the response TODO determine if successful
    let resp = Response {
        req_id: event.context.request_id,
        successful: true,
    };
    Ok(resp)
}

/// Application entrypoint
///
/// Runs the main handler
#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
