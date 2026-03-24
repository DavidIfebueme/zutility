use zutility_be::config::{AppConfig, AppEnv, ZcashNetwork, ZcashRpcMode};

fn valid_builder() -> config::ConfigBuilder<config::builder::DefaultState> {
    config::Config::builder()
        .set_override("app_env", "dev")
        .expect("set app_env")
        .set_override("http_bind_addr", "127.0.0.1:3001")
        .expect("set http_bind_addr")
        .set_override(
            "database_url",
            "postgres://postgres:postgres@localhost:5432/zutility",
        )
        .expect("set database_url")
        .set_override("order_token_hmac_secret", "order_secret")
        .expect("set order_token_hmac_secret")
        .set_override("ip_hash_secret", "ip_secret")
        .expect("set ip_hash_secret")
        .set_override("vtpass_base_url", "https://sandbox.vtpass.com/api")
        .expect("set vtpass_base_url")
        .set_override("vtpass_api_key", "vtpass_api_key")
        .expect("set vtpass_api_key")
        .set_override("vtpass_secret_key", "vtpass_secret_key")
        .expect("set vtpass_secret_key")
        .set_override("zcash_rpc_mode", "unix")
        .expect("set zcash_rpc_mode")
        .set_override("zcash_rpc_socket_path", "/var/run/zcashd/zcashd.sock")
        .expect("set zcash_rpc_socket_path")
        .set_override("zcash_rpc_url", "http://127.0.0.1:18232")
        .expect("set zcash_rpc_url")
        .set_override("zcash_rpc_user", "rpc_user")
        .expect("set zcash_rpc_user")
        .set_override("zcash_rpc_password", "rpc_password")
        .expect("set zcash_rpc_password")
        .set_override("zcash_network", "testnet")
        .expect("set zcash_network")
        .set_override("required_confs_transparent", 3)
        .expect("set required_confs_transparent")
        .set_override("required_confs_shielded", 10)
        .expect("set required_confs_shielded")
        .set_override("order_expiry_minutes", 30)
        .expect("set order_expiry_minutes")
        .set_override("rate_lock_minutes", 15)
        .expect("set rate_lock_minutes")
        .set_override("sweep_threshold_zec", "0.5")
        .expect("set sweep_threshold_zec")
        .set_override("signing_service_url", "http://10.0.0.2:8080")
        .expect("set signing_service_url")
        .set_override("signing_service_hmac_secret", "signing_secret")
        .expect("set signing_service_hmac_secret")
        .set_override("rate_source_timeout_ms", 3000)
        .expect("set rate_source_timeout_ms")
}

#[test]
fn loads_complete_contract_and_validates() {
    let config_map = valid_builder().build().expect("build config");
    let config = AppConfig::from_config(config_map).expect("deserialize config");

    assert!(matches!(config.app_env, AppEnv::Dev));
    assert!(matches!(config.zcash_rpc_mode, ZcashRpcMode::Unix));
    assert!(matches!(config.zcash_network, ZcashNetwork::Testnet));
    assert_eq!(config.order_expiry_minutes, 30);
    assert_eq!(config.rate_lock_minutes, 15);
    assert!(config.validate().is_ok());
}

#[test]
fn fails_validation_when_rate_lock_exceeds_expiry() {
    let config_map = valid_builder()
        .set_override("rate_lock_minutes", 31)
        .expect("set override")
        .build()
        .expect("build config");

    let config = AppConfig::from_config(config_map).expect("deserialize config");
    assert!(config.validate().is_err());
}

#[test]
fn fails_validation_when_secret_is_empty() {
    let config_map = valid_builder()
        .set_override("order_token_hmac_secret", "")
        .expect("set override")
        .build()
        .expect("build config");

    let config = AppConfig::from_config(config_map).expect("deserialize config");
    assert!(config.validate().is_err());
}

#[test]
fn fails_validation_for_unix_mode_without_socket_path() {
    let config_map = valid_builder()
        .set_override("zcash_rpc_mode", "unix")
        .expect("set override")
        .set_override("zcash_rpc_socket_path", "")
        .expect("set override")
        .build()
        .expect("build config");

    let config = AppConfig::from_config(config_map).expect("deserialize config");
    assert!(config.validate().is_err());
}

#[test]
fn fails_validation_for_tcp_mode_without_url() {
    let config_map = valid_builder()
        .set_override("zcash_rpc_mode", "tcp")
        .expect("set override")
        .set_override("zcash_rpc_url", "")
        .expect("set override")
        .build()
        .expect("build config");

    let config = AppConfig::from_config(config_map).expect("deserialize config");
    assert!(config.validate().is_err());
}
