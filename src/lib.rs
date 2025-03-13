use flowsnet_platform_sdk::logger;
use serde_json::{json, Value};
use std::collections::HashMap;
use webhook_flows::{create_endpoint, request_handler, send_response};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use store::*;
use store_flows as store;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    create_endpoint().await;
}

#[request_handler]
async fn handler(
    _headers: Vec<(String, String)>,
    _subpath: String,
    qry: HashMap<String, Value>,
    body: Vec<u8>,
) {
    logger::init();

    match _subpath.as_str() {
        // 🔥 处理 GitHub OAuth 回调
        "/auth/callback" => {
            handle_auth_callback(qry).await;
        }
        // 🔥 处理 GitHub OAuth 轮询接口
        "/auth/status" => {
            handle_check_status(qry).await;
        }
        // 🔥 处理静态文件
        "/index.html" | "/index" => {
            send_response(
                200,
                vec![(String::from("content-type"), String::from("text/html"))],
                include_str!("index.html").as_bytes().to_vec(),
            );
        }
        "/favicon.ico" => {
            send_response(
                200,
                vec![(String::from("content-type"), String::from("image/x-icon"))],
                include_bytes!("favicon.ico").to_vec(),
            );
        }
        _ => {
            send_response(
                404,
                vec![(String::from("content-type"), String::from("text/plain"))],
                b"Not Found".to_vec(),
            );
        }
    }
}

// ✅ 处理 GitHub OAuth 回调
async fn handle_auth_callback(qry: HashMap<String, Value>) {
    if let Some(code) = qry.get("code").and_then(|v| v.as_str()) {
        let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap();
        let client_secret = std::env::var("GITHUB_CLIENT_SECRET").unwrap();

        let client = Client::new();
        let params = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
        ];

        match client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
        {
            Ok(response) => match response.json::<serde_json::Value>().await {
                Ok(token_response) => {
                    if let Some(token) = token_response.get("access_token").and_then(|v| v.as_str()) {
                        let session_id = Uuid::new_v4().to_string();

                        store::set(
                            &session_id,
                            serde_json::to_value(token).unwrap(),
                            Some(Expire {
                                kind: ExpireKind::Ex,
                                value: 120,
                            }),
                        );

                        let redirect_url = format!(
                            "http://localhost:3000/login-success.html?session_id={}",
                            session_id
                        );

                        send_response(
                            302,
                            vec![(
                                String::from("Location"),
                                redirect_url,
                            )],
                            Vec::new(),
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse token response: {:?}", e);
                    send_response(
                        500,
                        vec![(String::from("content-type"), String::from("text/plain"))],
                        b"Failed to parse token response".to_vec(),
                    );
                }
            },
            Err(e) => {
                eprintln!("Failed to exchange token: {:?}", e);
                send_response(
                    500,
                    vec![(String::from("content-type"), String::from("text/plain"))],
                    b"Failed to exchange token".to_vec(),
                );
            }
        }
    } else {
        send_response(
            400,
            vec![(String::from("content-type"), String::from("text/plain"))],
            b"Invalid code".to_vec(),
        );
    }
}

// ✅ 处理 GitHub OAuth 轮询接口
async fn handle_check_status(qry: HashMap<String, Value>) {
    if let Some(session_id) = qry.get("session_id").and_then(|v| v.as_str()) {
        let token = match store::get(&session_id) {
            Some(v) => v,
            None => {
                status = false;
                message = "The token is wrong!";
                Value::Null
            }
        };

        if let Some(token) = token {
            let response = json!({
                "status": "authorized",
                "token": token
            });
            send_response(
                200,
                vec![(String::from("content-type"), String::from("application/json"))],
                serde_json::to_string(&response).unwrap().as_bytes().to_vec(),
            );
        } else {
            let response = json!({
                "status": "pending",
                "token": null
            });
            send_response(
                200,
                vec![(String::from("content-type"), String::from("application/json"))],
                serde_json::to_string(&response).unwrap().as_bytes().to_vec(),
            );
        }
    } else {
        send_response(
            400,
            vec![(String::from("content-type"), String::from("application/json"))],
            b"Invalid session ID".to_vec(),
        );
    }
}
