use rust_decimal::{Decimal, prelude::ToPrimitive};
use thiserror::Error;

use super::money::Money;

pub const AMOUNT_KIND_FIXED: &str = "fixed";
pub const AMOUNT_KIND_RANGED: &str = "ranged";
pub const AMOUNT_KIND_VARIATION: &str = "variation";

pub const MAX_ORDER_AMOUNT_NGN: i64 = 5_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PricingError {
    #[error("that product is not available right now")]
    ProductUnavailable,
    #[error("the settlement currency does not match the product")]
    CurrencyMismatch,
    #[error("the settlement amount is outside the product bounds")]
    AmountOutOfBounds,
    #[error("the settlement amount is not on the product increment")]
    AmountOffIncrement,
    #[error("a variation code is required for this product")]
    VariationRequired,
    #[error("that variation is not available")]
    VariationUnknown,
    #[error("the settlement amount does not match the variation")]
    VariationMismatch,
    #[error("amount must be greater than zero")]
    NonPositiveAmount,
    #[error("the order amount is above the supported maximum")]
    AmountAboveMaximum,
    #[error("the product price list is stale")]
    CatalogueStale,
}

#[derive(Debug, Clone)]
pub struct ProductTerms {
    pub amount_kind: String,
    pub amount_min: Option<Decimal>,
    pub amount_max: Option<Decimal>,
    pub amount_increment: Option<Decimal>,
    pub destination_currency: String,
    pub wholesale_rate: Decimal,
    pub wholesale_fee: Decimal,
    pub retail_rate: Option<Decimal>,
    pub retail_fee: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PricedOrder {
    pub amount_ngn: i64,
    pub cost_amount: Decimal,
}

pub fn validate_settlement(
    terms: &ProductTerms,
    settlement: Money,
    variation_amount: Option<Decimal>,
) -> Result<(), PricingError> {
    if settlement.currency().iso() != terms.destination_currency.trim() {
        return Err(PricingError::CurrencyMismatch);
    }

    if settlement.amount() <= Decimal::ZERO {
        return Err(PricingError::NonPositiveAmount);
    }

    match terms.amount_kind.as_str() {
        AMOUNT_KIND_VARIATION => {
            let Some(variation_amount) = variation_amount else {
                return Err(PricingError::VariationRequired);
            };
            if variation_amount != settlement.amount() {
                return Err(PricingError::VariationMismatch);
            }
            Ok(())
        }
        AMOUNT_KIND_FIXED => match terms.amount_min {
            Some(amount_min) if amount_min == settlement.amount() => Ok(()),
            _ => Err(PricingError::AmountOutOfBounds),
        },
        AMOUNT_KIND_RANGED => {
            let Some(amount_min) = terms.amount_min else {
                return Err(PricingError::AmountOutOfBounds);
            };
            let Some(amount_max) = terms.amount_max else {
                return Err(PricingError::AmountOutOfBounds);
            };

            if settlement.amount() < amount_min || settlement.amount() > amount_max {
                return Err(PricingError::AmountOutOfBounds);
            }

            if let Some(increment) = terms.amount_increment
                && increment > Decimal::ZERO
                && !(settlement.amount() - amount_min).is_zero()
                && !((settlement.amount() - amount_min) % increment).is_zero()
            {
                return Err(PricingError::AmountOffIncrement);
            }

            Ok(())
        }
        _ => Err(PricingError::ProductUnavailable),
    }
}

pub fn price(terms: &ProductTerms, settlement: Money) -> Result<PricedOrder, PricingError> {
    let rate = terms.retail_rate.unwrap_or(terms.wholesale_rate);
    let fee = match terms.retail_rate {
        Some(_) => terms.retail_fee.unwrap_or(Decimal::ZERO),
        None => terms.wholesale_fee,
    };

    let gross = settlement.amount() * rate + fee;
    let amount_ngn = gross
        .ceil()
        .to_i64()
        .ok_or(PricingError::AmountAboveMaximum)?;

    if amount_ngn <= 0 {
        return Err(PricingError::NonPositiveAmount);
    }
    if amount_ngn > MAX_ORDER_AMOUNT_NGN {
        return Err(PricingError::AmountAboveMaximum);
    }

    let cost_amount =
        (settlement.amount() * terms.wholesale_rate + terms.wholesale_fee).round_dp(4);

    Ok(PricedOrder {
        amount_ngn,
        cost_amount,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::money::CurrencyCode;

    fn nigerian_terms(kind: &str) -> ProductTerms {
        ProductTerms {
            amount_kind: String::from(kind),
            amount_min: Some(Decimal::from(50)),
            amount_max: Some(Decimal::from(50_000)),
            amount_increment: Some(Decimal::from(50)),
            destination_currency: String::from("NGN"),
            wholesale_rate: Decimal::ONE,
            wholesale_fee: Decimal::ZERO,
            retail_rate: None,
            retail_fee: None,
        }
    }

    fn senegalese_terms() -> ProductTerms {
        ProductTerms {
            amount_kind: String::from("ranged"),
            amount_min: Some(Decimal::from(500)),
            amount_max: Some(Decimal::from(100_000)),
            amount_increment: Some(Decimal::from(500)),
            destination_currency: String::from("XOF"),
            wholesale_rate: Decimal::new(120, 2),
            wholesale_fee: Decimal::from(50),
            retail_rate: Some(Decimal::new(130, 2)),
            retail_fee: Some(Decimal::from(25)),
        }
    }

    #[test]
    fn a_nigerian_row_prices_at_the_validated_amount() {
        let terms = nigerian_terms("ranged");
        let settlement = Money::new(CurrencyCode::Ngn, Decimal::from(5000)).expect("money");
        validate_settlement(&terms, settlement, None).expect("valid");
        let priced = price(&terms, settlement).expect("price");
        assert_eq!(priced.amount_ngn, 5000);
        assert_eq!(priced.cost_amount, Decimal::from(5000));
    }

    #[test]
    fn a_senegal_row_prices_retail_and_costs_wholesale() {
        let terms = senegalese_terms();
        let settlement = Money::new(CurrencyCode::Xof, Decimal::from(1000)).expect("money");
        validate_settlement(&terms, settlement, None).expect("valid");
        let priced = price(&terms, settlement).expect("price");
        assert_eq!(priced.amount_ngn, 1325);
        assert_eq!(priced.cost_amount, Decimal::from(1250));
    }

    #[test]
    fn an_off_increment_amount_is_rejected() {
        let terms = nigerian_terms("ranged");
        let settlement = Money::new(CurrencyCode::Ngn, Decimal::from(5125)).expect("money");
        assert_eq!(
            validate_settlement(&terms, settlement, None),
            Err(PricingError::AmountOffIncrement)
        );
    }

    #[test]
    fn a_fixed_product_requires_its_exact_amount() {
        let mut terms = nigerian_terms("fixed");
        terms.amount_min = Some(Decimal::from(6500));
        terms.amount_max = Some(Decimal::from(6500));
        terms.amount_increment = None;

        let wrong = Money::new(CurrencyCode::Ngn, Decimal::from(6000)).expect("money");
        assert_eq!(
            validate_settlement(&terms, wrong, None),
            Err(PricingError::AmountOutOfBounds)
        );

        let right = Money::new(CurrencyCode::Ngn, Decimal::from(6500)).expect("money");
        assert!(validate_settlement(&terms, right, None).is_ok());
    }

    #[test]
    fn a_variation_requires_a_matching_tier() {
        let terms = nigerian_terms("variation");
        let settlement = Money::new(CurrencyCode::Ngn, Decimal::from(1500)).expect("money");

        assert_eq!(
            validate_settlement(&terms, settlement, None),
            Err(PricingError::VariationRequired)
        );
        assert_eq!(
            validate_settlement(&terms, settlement, Some(Decimal::from(1000))),
            Err(PricingError::VariationMismatch)
        );
        assert!(validate_settlement(&terms, settlement, Some(Decimal::from(1500))).is_ok());
    }

    #[test]
    fn a_currency_mismatch_is_rejected() {
        let terms = senegalese_terms();
        let settlement = Money::new(CurrencyCode::Ngn, Decimal::from(1000)).expect("money");
        assert_eq!(
            validate_settlement(&terms, settlement, None),
            Err(PricingError::CurrencyMismatch)
        );
    }
}
