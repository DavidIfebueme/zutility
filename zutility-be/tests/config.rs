use zutility_be::config::{AppConfig, AppEnv};

#[test]
fn loads_config_from_builder_source() {
    let config_map = config::Config::builder()
        .set_override("app_env", "dev")
        .expect("app_env should be set")
        .set_override("http_bind_addr", "127.0.0.1:3001")
        .expect("http_bind_addr should be set")
        .set_override(
            "database_url",
            "postgres://postgres:postgres@localhost:5432/zutility",
        )
        .expect("database_url should be set")
        .build()
        .expect("config builder should build");

    let config = AppConfig::from_config(config_map).expect("config should load");

    assert!(matches!(config.app_env, AppEnv::Dev));
    assert_eq!(config.http_bind_addr.to_string(), "127.0.0.1:3001");
    assert_eq!(
        config.database_url,
        "postgres://postgres:postgres@localhost:5432/zutility"
    );
    config.validate().expect("config should validate");
}
