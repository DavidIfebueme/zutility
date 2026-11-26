#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rust_decimal::Decimal;
use zutility_be::domain::money::{CurrencyCode, Money, MoneyError};

fn parsed(currency: CurrencyCode, raw: &str) -> Option<Money> {
    Money::from_major(currency, raw).ok()
}

#[test]
fn ngn_whole_naira_round_trips() {
    let money = Money::from_ngn_whole(5000).ok();
    assert_eq!(
        money.and_then(|value| value.to_ngn_whole().ok()),
        Some(5000)
    );
}

#[test]
fn xof_has_no_decimals() {
    let formatted =
        parsed(CurrencyCode::Xof, "1000").map(|value| value.round_to_currency().to_string());
    assert_eq!(formatted.as_deref(), Some("1000"));
    assert_eq!(CurrencyCode::Xof.exponent(), 0);
}

#[test]
fn xof_rounds_half_away_from_zero() {
    let down =
        parsed(CurrencyCode::Xof, "1000.4").map(|value| value.round_to_currency().to_string());
    let up = parsed(CurrencyCode::Xof, "1000.5").map(|value| value.round_to_currency().to_string());
    assert_eq!(down.as_deref(), Some("1000"));
    assert_eq!(up.as_deref(), Some("1001"));
}

#[test]
fn zar_keeps_two_decimals() {
    let formatted =
        parsed(CurrencyCode::Zar, "10.50").map(|value| value.round_to_currency().to_string());
    assert_eq!(formatted.as_deref(), Some("10.50"));
}

#[test]
fn zar_midpoint_rounds_up() {
    let rounded =
        parsed(CurrencyCode::Zar, "10.505").map(|value| value.round_to_currency().to_string());
    assert_eq!(rounded.as_deref(), Some("10.51"));
}

#[test]
fn negative_amounts_are_rejected() {
    assert_eq!(
        Money::from_major(CurrencyCode::Ngn, "-1").err(),
        Some(MoneyError::Negative)
    );
    assert_eq!(Money::from_ngn_whole(-1).err(), Some(MoneyError::Negative));
}

#[test]
fn over_precise_amounts_are_rejected_by_new() {
    let amount = "10.505".parse::<Decimal>();
    assert!(amount.is_ok());
    if let Ok(value) = amount {
        assert_eq!(
            Money::new(CurrencyCode::Zar, value).err(),
            Some(MoneyError::TooPrecise)
        );
    }
}

#[test]
fn overflow_is_rejected() {
    assert_eq!(
        Money::from_major(CurrencyCode::Ngn, "1000000000001").err(),
        Some(MoneyError::Overflow)
    );
}

#[test]
fn unknown_iso_codes_are_rejected() {
    assert_eq!(CurrencyCode::from_iso("XXX"), None);
    assert_eq!(CurrencyCode::from_iso("xof"), Some(CurrencyCode::Xof));
    assert_eq!(CurrencyCode::ALL.len(), 10);
}

#[test]
fn ngn_converts_through_usd() {
    let naira = Money::from_ngn_whole(150_000);
    assert!(naira.is_ok());
    if let Ok(value) = naira {
        let converted =
            value.display_in(CurrencyCode::Xof, Decimal::from(1500), Decimal::new(6, 1));
        assert_eq!(
            converted.map(|money| money.round_to_currency().to_string()),
            Some("60".to_string())
        );
    }
}

#[test]
fn conversion_rejects_a_zero_rate() {
    let naira = Money::from_ngn_whole(150_000);
    assert!(naira.is_ok());
    if let Ok(value) = naira {
        assert_eq!(
            value.display_in(CurrencyCode::Xof, Decimal::ZERO, Decimal::new(6, 1)),
            None
        );
        assert_eq!(
            value.display_in(CurrencyCode::Xof, Decimal::from(1500), Decimal::ZERO),
            None
        );
    }
}

#[test]
fn display_includes_the_currency() {
    let formatted = parsed(CurrencyCode::Xof, "1000").map(|value| value.to_string());
    assert_eq!(formatted.as_deref(), Some("1000 XOF"));
}

#[tokio::test]
async fn user_currency_round_trips_through_every_query() {
    let Some(url) = std::env::var("TEST_DATABASE_URL").ok() else {
        return;
    };
    let pool = sqlx::PgPool::connect(&url)
        .await
        .expect("TEST_DATABASE_URL is set but the test database is unreachable");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("test database migrations failed");

    let email = format!("money-{}@example.com", uuid::Uuid::new_v4());
    let created = zutility_be::db::create_user(&pool, &email, Some("probe"), "hash", "XOF")
        .await
        .expect("create user");
    assert_eq!(created.preferred_currency, "XOF");

    let by_email = zutility_be::db::find_user_by_email(&pool, &email)
        .await
        .expect("find by email")
        .map(|user| user.preferred_currency);
    assert_eq!(by_email.as_deref(), Some("XOF"));

    let by_id = zutility_be::db::find_user_by_id(&pool, created.id)
        .await
        .expect("find by id")
        .map(|user| user.preferred_currency);
    assert_eq!(by_id.as_deref(), Some("XOF"));

    zutility_be::db::update_user_preferred_currency(&pool, created.id, "GHS")
        .await
        .expect("update currency");
    let updated = zutility_be::db::find_user_by_id(&pool, created.id)
        .await
        .expect("find updated")
        .map(|user| user.preferred_currency);
    assert_eq!(updated.as_deref(), Some("GHS"));

    zutility_be::db::update_user_display_name(&pool, created.id, None)
        .await
        .expect("preserve display name");
    let preserved = zutility_be::db::find_user_by_id(&pool, created.id)
        .await
        .expect("find preserved")
        .and_then(|user| user.display_name);
    assert_eq!(preserved.as_deref(), Some("probe"));
}

#[test]
fn conversion_returns_none_instead_of_panicking_on_an_overflowing_rate() {
    let ngn = Money::from_ngn_whole(1000).expect("naira");
    let tiny = Decimal::new(1, 28);
    assert_eq!(
        ngn.display_in(CurrencyCode::Xof, tiny, Decimal::ONE),
        None
    );
}
