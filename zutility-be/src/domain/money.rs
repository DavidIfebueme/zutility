use std::fmt;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

const MAX_WHOLE_UNITS: i64 = 1_000_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoneyError {
    Invalid,
    Negative,
    TooPrecise,
    Overflow,
    UnsupportedCurrency,
}

impl fmt::Display for MoneyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => write!(formatter, "amount is not a valid number"),
            Self::Negative => write!(formatter, "amount cannot be negative"),
            Self::TooPrecise => write!(
                formatter,
                "amount has more decimals than the currency allows"
            ),
            Self::Overflow => write!(formatter, "amount is too large"),
            Self::UnsupportedCurrency => {
                write!(formatter, "currency not supported for this operation")
            }
        }
    }
}

impl std::error::Error for MoneyError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CurrencyCode {
    Ngn,
    Xof,
    Gmd,
    Mwk,
    Zmw,
    Zar,
    Usd,
    Kes,
    Ghs,
    Egp,
}

impl CurrencyCode {
    pub const ALL: [CurrencyCode; 10] = [
        Self::Ngn,
        Self::Xof,
        Self::Gmd,
        Self::Mwk,
        Self::Zmw,
        Self::Zar,
        Self::Usd,
        Self::Kes,
        Self::Ghs,
        Self::Egp,
    ];

    pub fn iso(self) -> &'static str {
        match self {
            Self::Ngn => "NGN",
            Self::Xof => "XOF",
            Self::Gmd => "GMD",
            Self::Mwk => "MWK",
            Self::Zmw => "ZMW",
            Self::Zar => "ZAR",
            Self::Usd => "USD",
            Self::Kes => "KES",
            Self::Ghs => "GHS",
            Self::Egp => "EGP",
        }
    }

    pub fn exponent(self) -> u32 {
        match self {
            Self::Xof => 0,
            _ => 2,
        }
    }

    pub fn from_iso(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "NGN" => Some(Self::Ngn),
            "XOF" => Some(Self::Xof),
            "GMD" => Some(Self::Gmd),
            "MWK" => Some(Self::Mwk),
            "ZMW" => Some(Self::Zmw),
            "ZAR" => Some(Self::Zar),
            "USD" => Some(Self::Usd),
            "KES" => Some(Self::Kes),
            "GHS" => Some(Self::Ghs),
            "EGP" => Some(Self::Egp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money {
    currency: CurrencyCode,
    amount: Decimal,
}

impl Money {
    pub fn new(currency: CurrencyCode, amount: Decimal) -> Result<Self, MoneyError> {
        if amount.is_sign_negative() {
            return Err(MoneyError::Negative);
        }
        if amount.scale() > currency.exponent() {
            return Err(MoneyError::TooPrecise);
        }
        if amount > Decimal::from(MAX_WHOLE_UNITS) {
            return Err(MoneyError::Overflow);
        }
        Ok(Self { currency, amount })
    }

    pub fn new_rounded(currency: CurrencyCode, amount: Decimal) -> Result<Self, MoneyError> {
        if amount.is_sign_negative() {
            return Err(MoneyError::Negative);
        }
        let rounded = amount
            .round_dp_with_strategy(currency.exponent(), RoundingStrategy::MidpointAwayFromZero);
        if rounded > Decimal::from(MAX_WHOLE_UNITS) {
            return Err(MoneyError::Overflow);
        }
        Ok(Self {
            currency,
            amount: rounded,
        })
    }

    pub fn from_major(currency: CurrencyCode, raw: &str) -> Result<Self, MoneyError> {
        let amount = raw
            .trim()
            .parse::<Decimal>()
            .map_err(|_| MoneyError::Invalid)?;
        Self::new_rounded(currency, amount)
    }

    pub fn from_ngn_whole(naira: i64) -> Result<Self, MoneyError> {
        if naira < 0 {
            return Err(MoneyError::Negative);
        }
        Self::new(CurrencyCode::Ngn, Decimal::from(naira))
    }

    pub fn currency(&self) -> CurrencyCode {
        self.currency
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn round_to_currency(&self) -> Decimal {
        self.amount.round_dp_with_strategy(
            self.currency.exponent(),
            RoundingStrategy::MidpointAwayFromZero,
        )
    }

    pub fn to_ngn_whole(&self) -> Result<i64, MoneyError> {
        if self.currency != CurrencyCode::Ngn {
            return Err(MoneyError::UnsupportedCurrency);
        }
        self.amount
            .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
            .to_i64()
            .ok_or(MoneyError::Overflow)
    }

    pub fn convert_from_ngn(
        self,
        target: CurrencyCode,
        usd_ngn: Decimal,
        target_per_usd: Decimal,
    ) -> Option<Self> {
        if self.currency != CurrencyCode::Ngn {
            return None;
        }
        if usd_ngn <= Decimal::ZERO || target_per_usd <= Decimal::ZERO {
            return None;
        }
        if target == CurrencyCode::Ngn {
            return Some(self);
        }
        let usd = self.amount / usd_ngn;
        Self::new_rounded(target, usd * target_per_usd).ok()
    }
}

impl fmt::Display for Money {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}",
            self.round_to_currency(),
            self.currency.iso()
        )
    }
}
