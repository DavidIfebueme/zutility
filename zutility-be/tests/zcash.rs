use std::{sync::Arc, time::Duration};

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use secrecy::SecretString;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use zutility_be::{
    config::{
        AppConfig, AppEnv, ZcashNetwork as ConfigZcashNetwork, ZcashRpcMode as ConfigZcashRpcMode,
    },
    integrations::zcash::{
        RpcRetryPolicy, ZcashNetwork, ZcashRpcClient, ZcashRpcConfig, ZcashRpcMode,
        validate_rpc_socket_policy, validate_runtime_network_policy,
    },
};

#[derive(Clone)]
struct MockRpcState {
    requests: Arc<Mutex<Vec<String>>>,
    fail_first_blockchain_info: bool,
    use_mainnet_chain: bool,
    fail_account_api: bool,
}

async fn rpc_handler(
    State(state): State<MockRpcState>,
    Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let method = payload
        .get("method")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_default();

    {
        let mut requests = state.requests.lock().await;
        requests.push(method.clone());
    }

    if method == "getblockchaininfo" && state.fail_first_blockchain_info {
        let count = state
            .requests
            .lock()
            .await
            .iter()
            .filter(|m| m.as_str() == "getblockchaininfo")
            .count();
        if count == 1 {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "result": null,
                    "error": { "code": -1, "message": "temporary failure" },
                    "id": "1"
                })),
            );
        }
    }

    let response = match method.as_str() {
        "getblockchaininfo" => {
            let chain = if state.use_mainnet_chain {
                "main"
            } else {
                "test"
            };
            json!({
                "result": {
                    "chain": chain,
                    "blocks": 100,
                    "headers": 100,
                    "verificationprogress": 0.99
                },
                "error": null,
                "id": "1"
            })
        }
        "z_getnewaccount" => {
            if state.fail_account_api {
                json!({
                    "result": null,
                    "error": { "code": -32601, "message": "method not found" },
                    "id": "1"
                })
            } else {
                json!({ "result": 7, "error": null, "id": "1" })
            }
        }
        "z_getaddressforaccount" => json!({
            "result": "ztestsapling1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq",
            "error": null,
            "id": "1"
        }),
        "z_getnewaddress" => json!({
            "result": "ztestsapling1fallbackfallbackfallbackfallbackfallbackfallbackfallbackfallbackfallbackfallback",
            "error": null,
            "id": "1"
        }),
        "z_listreceivedbyaddress" => json!({
            "result": [
                {
                    "txid": "tx-1",
                    "address": "ztestsapling1abc",
                    "amount": "0.10000000",
                    "confirmations": 1,
                    "memo": "first"
                },
                {
                    "txid": "tx-2",
                    "address": "ztestsapling1abc",
                    "amount": "0.25000000",
                    "confirmations": 6,
                    "memo": "second"
                }
            ],
            "error": null,
            "id": "1"
        }),
        _ => json!({
            "result": null,
            "error": { "code": -32601, "message": "unknown method" },
            "id": "1"
        }),
    };

    (StatusCode::OK, Json(response))
}

async fn spawn_mock_rpc(state: MockRpcState) -> String {
    let app = Router::new()
        .route("/", post(rpc_handler))
        .with_state(state);
    let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
        Ok(listener) => listener,
        Err(error) => panic!("failed to bind mock rpc listener: {error}"),
    };
    let addr = match listener.local_addr() {
        Ok(addr) => addr,
        Err(error) => panic!("failed to get local addr: {error}"),
    };

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    format!("http://{addr}")
}

fn build_client(url: String) -> ZcashRpcClient {
    let config = ZcashRpcConfig {
        mode: ZcashRpcMode::Tcp,
        socket_path: String::new(),
        rpc_url: url,
        rpc_user: String::from("rpcuser"),
        rpc_password: String::from("rpcpass"),
        network: ZcashNetwork::Testnet,
    };

    match ZcashRpcClient::new(
        config,
        RpcRetryPolicy {
            max_retries: 2,
            timeout: Duration::from_secs(2),
        },
    ) {
        Ok(client) => client,
        Err(error) => panic!("failed to build zcash client: {error}"),
    }
}

#[tokio::test]
async fn retries_and_health_check_pass_on_testnet() {
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let url = spawn_mock_rpc(MockRpcState {
        requests: Arc::clone(&requests),
        fail_first_blockchain_info: true,
        use_mainnet_chain: false,
        fail_account_api: false,
    })
    .await;

    let client = build_client(url);
    let info = client.health_check_testnet(0.95).await;
    assert!(info.is_ok());

    let count = requests
        .lock()
        .await
        .iter()
        .filter(|method| method.as_str() == "getblockchaininfo")
        .count();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn health_check_fails_for_mainnet_chain() {
    let url = spawn_mock_rpc(MockRpcState {
        requests: Arc::new(Mutex::new(Vec::<String>::new())),
        fail_first_blockchain_info: false,
        use_mainnet_chain: true,
        fail_account_api: false,
    })
    .await;

    let client = build_client(url);
    let result = client.health_check_testnet(0.95).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn account_api_falls_back_to_deprecated_when_enabled() {
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let url = spawn_mock_rpc(MockRpcState {
        requests: Arc::clone(&requests),
        fail_first_blockchain_info: false,
        use_mainnet_chain: false,
        fail_account_api: true,
    })
    .await;

    let client = build_client(url);
    let address = client.allocate_shielded_address(true).await;
    assert!(address.is_ok());

    let requests = requests.lock().await;
    assert!(requests.iter().any(|m| m == "z_getnewaccount"));
    assert!(requests.iter().any(|m| m == "z_getnewaddress"));
}

#[tokio::test]
async fn list_received_filters_by_confirmations() {
    let url = spawn_mock_rpc(MockRpcState {
        requests: Arc::new(Mutex::new(Vec::<String>::new())),
        fail_first_blockchain_info: false,
        use_mainnet_chain: false,
        fail_account_api: false,
    })
    .await;

    let client = build_client(url);
    let notes = client.list_received_by_address("ztestsapling1abc", 3).await;
    assert!(notes.is_ok());
    let notes = notes.unwrap_or_default();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].txid, "tx-2");
}

#[test]
fn runtime_network_policy_enforces_testnet_for_dev_and_staging() {
    let dev_mainnet = AppConfig {
        app_env: AppEnv::Dev,
        http_bind_addr: "127.0.0.1:8080"
            .parse()
            .unwrap_or_else(|error| panic!("failed to parse bind addr: {error}")),
        database_url: String::from("postgres://postgres:postgres@localhost/zutility"),
        order_token_hmac_secret: SecretString::from(String::from("order-secret")),
        ip_hash_secret: SecretString::from(String::from("ip-secret")),
        vtpass_base_url: String::from("https://sandbox.vtpass.com"),
        vtpass_api_key: SecretString::from(String::from("key")),
        vtpass_secret_key: SecretString::from(String::from("secret")),
        zcash_rpc_mode: ConfigZcashRpcMode::Tcp,
        zcash_rpc_socket_path: String::new(),
        zcash_rpc_url: String::from("http://127.0.0.1:8232"),
        zcash_rpc_user: SecretString::from(String::from("rpcuser")),
        zcash_rpc_password: SecretString::from(String::from("rpcpass")),
        zcash_network: ConfigZcashNetwork::Mainnet,
        required_confs_transparent: 3,
        required_confs_shielded: 10,
        order_expiry_minutes: 15,
        rate_lock_minutes: 5,
        sweep_threshold_zec: rust_decimal::Decimal::new(1, 1),
        signing_service_url: String::from("http://localhost:9000"),
        signing_service_hmac_secret: SecretString::from(String::from("signing-secret")),
        rate_source_timeout_ms: 1500,
    };

    let result = validate_runtime_network_policy(&dev_mainnet);
    assert!(result.is_err());
}

#[test]
fn rpc_socket_policy_requires_unix_in_production() {
    let prod_tcp = AppConfig {
        app_env: AppEnv::Prod,
        http_bind_addr: "127.0.0.1:8080"
            .parse()
            .unwrap_or_else(|error| panic!("failed to parse bind addr: {error}")),
        database_url: String::from("postgres://postgres:postgres@localhost/zutility"),
        order_token_hmac_secret: SecretString::from(String::from("order-secret")),
        ip_hash_secret: SecretString::from(String::from("ip-secret")),
        vtpass_base_url: String::from("https://sandbox.vtpass.com"),
        vtpass_api_key: SecretString::from(String::from("key")),
        vtpass_secret_key: SecretString::from(String::from("secret")),
        zcash_rpc_mode: ConfigZcashRpcMode::Tcp,
        zcash_rpc_socket_path: String::new(),
        zcash_rpc_url: String::from("http://127.0.0.1:8232"),
        zcash_rpc_user: SecretString::from(String::from("rpcuser")),
        zcash_rpc_password: SecretString::from(String::from("rpcpass")),
        zcash_network: ConfigZcashNetwork::Mainnet,
        required_confs_transparent: 3,
        required_confs_shielded: 10,
        order_expiry_minutes: 15,
        rate_lock_minutes: 5,
        sweep_threshold_zec: rust_decimal::Decimal::new(1, 1),
        signing_service_url: String::from("http://localhost:9000"),
        signing_service_hmac_secret: SecretString::from(String::from("signing-secret")),
        rate_source_timeout_ms: 1500,
    };

    let result = validate_rpc_socket_policy(&prod_tcp);
    assert!(result.is_err());
}
