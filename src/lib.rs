use flowsnet_platform_sdk::logger;
use serde_json::Value;
use std::collections::HashMap;
use webhook_flows::{create_endpoint, request_handler, send_response};

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
    let html = match _subpath.as_str() {
        "/index.html" | "/index" => include_str!("index.html"),
        _ => include_str!("index.html")
    };
    send_response(
        200,
        vec![(String::from("content-type"), String::from("text/html"))],
        html.as_bytes().to_vec(),
    );
}