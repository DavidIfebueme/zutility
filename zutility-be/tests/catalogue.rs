#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use tower::util::ServiceExt;
use zutility_be::{domain::catalogue, http};

mod support;

async fn reset_catalogue(pool: &PgPool) {
    sqlx::query("UPDATE countries SET enabled = (iso2 = 'NG')")
        .execute(pool)
        .await
        .expect("reset country enablement");
    sqlx::query("UPDATE provider_products SET enabled = true")
        .execute(pool)
        .await
        .expect("reset product enablement");
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("build request");
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .expect("read body");
    let json = serde_json::from_slice(&body).expect("json parse");
    (status, json)
}

fn countries(body: &Value) -> Vec<&Value> {
    body.get("countries")
        .and_then(Value::as_array)
        .expect("countries array")
        .iter()
        .collect()
}

fn products<'a>(categories: &[&'a Value]) -> Vec<&'a Value> {
    categories
        .iter()
        .flat_map(|category| {
            category
                .get("products")
                .and_then(Value::as_array)
                .expect("products array")
        })
        .collect()
}

fn find_product<'a>(products: &[&'a Value], slug: &str) -> &'a Value {
    products
        .iter()
        .copied()
        .find(|product| product.get("slug").and_then(Value::as_str) == Some(slug))
        .expect("product present")
}

fn assert_no_rate_keys(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                assert!(
                    !key.contains("rate") && !key.contains("fee"),
                    "unexpected pricing key in catalogue response: {key}"
                );
                assert_no_rate_keys(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_rate_keys(item);
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn nigeria_returns_twenty_eight_products_in_six_categories() {
    let Some(pool) = support::pool().await else {
        return;
    };
    reset_catalogue(&pool).await;
    let app = http::router_with_pool(Some(pool));

    let (status, body) = get_json(&app, "/api/v1/catalogue").await;
    assert_eq!(status, StatusCode::OK);

    let countries = countries(&body);
    assert_eq!(countries.len(), 1);
    let nigeria = countries[0];
    assert_eq!(nigeria.get("iso2").and_then(Value::as_str), Some("NG"));
    assert_eq!(nigeria.get("name").and_then(Value::as_str), Some("Nigeria"));
    assert_eq!(nigeria.get("currency").and_then(Value::as_str), Some("NGN"));

    let categories = nigeria
        .get("categories")
        .and_then(Value::as_array)
        .expect("categories array");
    let category_refs: Vec<&Value> = categories.iter().collect();
    let names: Vec<&str> = categories
        .iter()
        .filter_map(|category| category.get("category").and_then(Value::as_str))
        .collect();
    assert_eq!(
        names,
        vec![
            "airtime",
            "data",
            "television",
            "electricity",
            "education",
            "school"
        ]
    );
    assert_eq!(products(&category_refs).len(), 28);

    assert_no_rate_keys(&body);
}

#[tokio::test]
async fn seeded_product_ids_are_deterministic_uuid_v5() {
    let Some(pool) = support::pool().await else {
        return;
    };
    reset_catalogue(&pool).await;

    let rows = sqlx::query("SELECT id, provider, provider_product_id FROM provider_products")
        .fetch_all(&pool)
        .await
        .expect("read seeded products");
    assert_eq!(rows.len(), 28);

    for row in rows {
        let id: uuid::Uuid = sqlx::Row::try_get(&row, "id").expect("id");
        let provider: String = sqlx::Row::try_get(&row, "provider").expect("provider");
        let provider_product_id: String =
            sqlx::Row::try_get(&row, "provider_product_id").expect("provider product id");
        assert_eq!(id, catalogue::product_id(&provider, &provider_product_id));
    }
}

#[tokio::test]
async fn airtime_is_ranged_between_fifty_and_fifty_thousand_naira() {
    let Some(pool) = support::pool().await else {
        return;
    };
    reset_catalogue(&pool).await;
    let app = http::router_with_pool(Some(pool));

    let (status, body) = get_json(&app, "/api/v1/catalogue?country=NG").await;
    assert_eq!(status, StatusCode::OK);
    let countries = countries(&body);
    let categories: Vec<&Value> = countries[0]
        .get("categories")
        .and_then(Value::as_array)
        .expect("categories")
        .iter()
        .collect();
    let product_list = products(&categories);
    let mtn = find_product(&product_list, "mtn");

    assert_eq!(
        mtn.get("amount_kind").and_then(Value::as_str),
        Some("ranged")
    );
    assert_eq!(mtn.get("amount_min").and_then(Value::as_str), Some("50"));
    assert_eq!(mtn.get("amount_max").and_then(Value::as_str), Some("50000"));
    assert_eq!(mtn.get("currency").and_then(Value::as_str), Some("NGN"));
}

#[tokio::test]
async fn jamb_is_fixed_at_six_thousand_five_hundred_naira() {
    let Some(pool) = support::pool().await else {
        return;
    };
    reset_catalogue(&pool).await;
    let app = http::router_with_pool(Some(pool));

    let (status, body) = get_json(&app, "/api/v1/catalogue?country=NG").await;
    assert_eq!(status, StatusCode::OK);
    let countries = countries(&body);
    let categories: Vec<&Value> = countries[0]
        .get("categories")
        .and_then(Value::as_array)
        .expect("categories")
        .iter()
        .collect();
    let product_list = products(&categories);
    let jamb = find_product(&product_list, "jamb");

    assert_eq!(
        jamb.get("amount_kind").and_then(Value::as_str),
        Some("fixed")
    );
    assert_eq!(jamb.get("amount_min").and_then(Value::as_str), Some("6500"));
    assert_eq!(jamb.get("amount_max").and_then(Value::as_str), Some("6500"));
}

#[tokio::test]
async fn disabled_country_returns_nothing() {
    let Some(pool) = support::pool().await else {
        return;
    };
    reset_catalogue(&pool).await;
    let app = http::router_with_pool(Some(pool.clone()));

    let (status, body) = get_json(&app, "/api/v1/catalogue?country=SN").await;
    assert_eq!(status, StatusCode::OK);
    assert!(countries(&body).is_empty());

    reset_catalogue(&pool).await;
}

#[tokio::test]
async fn disabled_country_with_enabled_products_returns_nothing() {
    let Some(pool) = support::pool().await else {
        return;
    };
    reset_catalogue(&pool).await;

    sqlx::query("UPDATE countries SET enabled = false WHERE iso2 = 'NG'")
        .execute(&pool)
        .await
        .expect("disable nigeria");

    let enabled_products: i64 =
        sqlx::query_scalar("SELECT count(*) FROM provider_products WHERE enabled")
            .fetch_one(&pool)
            .await
            .expect("count enabled products");
    assert_eq!(enabled_products, 28);

    let app = http::router_with_pool(Some(pool.clone()));
    let (status, body) = get_json(&app, "/api/v1/catalogue").await;
    assert_eq!(status, StatusCode::OK);
    assert!(countries(&body).is_empty());

    reset_catalogue(&pool).await;
}

#[tokio::test]
async fn unknown_country_returns_empty_countries_array() {
    let Some(pool) = support::pool().await else {
        return;
    };
    reset_catalogue(&pool).await;
    let app = http::router_with_pool(Some(pool));

    let (status, body) = get_json(&app, "/api/v1/catalogue?country=ZZ").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::json!({ "countries": [] }));
}

#[tokio::test]
async fn variation_products_carry_their_tiers() {
    let Some(pool) = support::pool().await else {
        return;
    };
    reset_catalogue(&pool).await;
    let app = http::router_with_pool(Some(pool));

    let (status, body) = get_json(&app, "/api/v1/catalogue?country=NG").await;
    assert_eq!(status, StatusCode::OK);
    let countries = countries(&body);
    let categories: Vec<&Value> = countries[0]
        .get("categories")
        .and_then(Value::as_array)
        .expect("categories")
        .iter()
        .collect();

    let mut variation_count = 0;
    for product in products(&categories) {
        let slug = product.get("slug").and_then(Value::as_str).expect("slug");
        let kind = product
            .get("amount_kind")
            .and_then(Value::as_str)
            .expect("amount_kind");
        let variations = product
            .get("variations")
            .and_then(Value::as_array)
            .expect("variations array");

        if kind == "variation" {
            variation_count += 1;
            assert!(!variations.is_empty(), "{slug} has no tiers");
            for variation in variations {
                let amount = variation
                    .get("amount")
                    .and_then(Value::as_str)
                    .expect("variation amount");
                let amount: f64 = amount.parse().expect("numeric variation amount");
                assert!(amount > 0.0, "{slug} has a non-positive tier");
            }
        } else {
            assert!(variations.is_empty(), "{slug} should not carry tiers");
        }
    }

    assert!(
        variation_count >= 10,
        "expected at least ten variation products, got {variation_count}"
    );
}

#[tokio::test]
async fn disabled_product_is_absent_from_its_country() {
    let Some(pool) = support::pool().await else {
        return;
    };
    reset_catalogue(&pool).await;
    sqlx::query("UPDATE provider_products SET enabled = false WHERE slug = 'mtn-data'")
        .execute(&pool)
        .await
        .expect("disable product");

    let app = http::router_with_pool(Some(pool.clone()));
    let (status, body) = get_json(&app, "/api/v1/catalogue?country=NG").await;
    assert_eq!(status, StatusCode::OK);
    let countries = countries(&body);
    let categories: Vec<&Value> = countries[0]
        .get("categories")
        .and_then(Value::as_array)
        .expect("categories")
        .iter()
        .collect();
    let slugs: Vec<&str> = products(&categories)
        .iter()
        .filter_map(|product| product.get("slug").and_then(Value::as_str))
        .collect();
    assert!(!slugs.contains(&"mtn-data"));

    sqlx::query("UPDATE provider_products SET enabled = true WHERE slug = 'mtn-data'")
        .execute(&pool)
        .await
        .expect("restore product");
}

#[tokio::test]
async fn catalogue_is_public_with_or_without_a_session() {
    let Some(app) = support::authed_router().await else {
        return;
    };

    let (status, body) = get_json(&app, "/api/v1/catalogue?country=NG").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(countries(&body).len(), 1);
}

#[tokio::test]
async fn legacy_utilities_endpoint_still_lists_twenty_eight_items() {
    let Some(pool) = support::pool().await else {
        return;
    };
    let app = http::router_with_pool(Some(pool));

    let (status, body) = get_json(&app, "/api/v1/utilities").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(28));
}
