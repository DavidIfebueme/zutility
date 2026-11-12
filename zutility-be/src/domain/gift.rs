use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

pub const MIN_CLAIM_WINDOW_HOURS: i64 = 12;
pub const MAX_CLAIM_WINDOW_HOURS: i64 = 168;
pub const MAX_GIFT_AMOUNT_NGN: i64 = 100_000;
pub const MAX_OUTSTANDING_GIFT_NGN: i64 = 500_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GiftStatus {
    Open,
    Claimed,
    Cancelled,
    Expired,
}

impl GiftStatus {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Claimed => "claimed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "claimed" => Some(Self::Claimed),
            "cancelled" => Some(Self::Cancelled),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Open, Self::Claimed)
                | (Self::Open, Self::Cancelled)
                | (Self::Open, Self::Expired)
        )
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum GiftValidationError {
    #[error(
        "claim window must be between {MIN_CLAIM_WINDOW_HOURS} and {MAX_CLAIM_WINDOW_HOURS} hours"
    )]
    InvalidClaimWindow,
    #[error("gift amount must be greater than zero")]
    NonPositiveAmount,
    #[error("gift amount cannot exceed {MAX_GIFT_AMOUNT_NGN}")]
    AmountAboveGiftMaximum,
}

pub fn validate_claim_window_hours(hours: i64) -> Result<(), GiftValidationError> {
    if (MIN_CLAIM_WINDOW_HOURS..=MAX_CLAIM_WINDOW_HOURS).contains(&hours) {
        return Ok(());
    }
    Err(GiftValidationError::InvalidClaimWindow)
}

pub fn validate_gift_amount_ngn(amount_ngn: i64) -> Result<(), GiftValidationError> {
    if amount_ngn <= 0 {
        return Err(GiftValidationError::NonPositiveAmount);
    }
    if amount_ngn > MAX_GIFT_AMOUNT_NGN {
        return Err(GiftValidationError::AmountAboveGiftMaximum);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldGiftOutcome {
    KeepHeld,
    ReleaseToDispatch,
    ResolveExpired,
    FlagForReview,
}

pub fn held_gift_outcome(
    status: GiftStatus,
    claim_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
) -> HeldGiftOutcome {
    match status {
        GiftStatus::Claimed => HeldGiftOutcome::ReleaseToDispatch,
        GiftStatus::Expired => HeldGiftOutcome::ResolveExpired,
        GiftStatus::Cancelled => HeldGiftOutcome::FlagForReview,
        GiftStatus::Open => match claim_expires_at {
            Some(deadline) if deadline <= now => HeldGiftOutcome::ResolveExpired,
            _ => HeldGiftOutcome::KeepHeld,
        },
    }
}

pub fn is_giftable_utility(utility_type: &str) -> bool {
    matches!(
        utility_type,
        "airtime"
            | "data"
            | "dstv"
            | "gotv"
            | "startimes"
            | "showmax"
            | "electricity"
            | "waec"
            | "jamb"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_allowed_claim_window() {
        assert!(validate_claim_window_hours(12).is_ok());
        assert!(validate_claim_window_hours(24).is_ok());
        assert!(validate_claim_window_hours(72).is_ok());
        assert!(validate_claim_window_hours(168).is_ok());
    }

    #[test]
    fn rejects_claim_window_outside_bounds() {
        assert_eq!(
            validate_claim_window_hours(11),
            Err(GiftValidationError::InvalidClaimWindow)
        );
        assert_eq!(
            validate_claim_window_hours(169),
            Err(GiftValidationError::InvalidClaimWindow)
        );
    }

    #[test]
    fn rejects_gift_amounts_above_the_cap() {
        assert!(validate_gift_amount_ngn(MAX_GIFT_AMOUNT_NGN).is_ok());
        assert_eq!(
            validate_gift_amount_ngn(MAX_GIFT_AMOUNT_NGN + 1),
            Err(GiftValidationError::AmountAboveGiftMaximum)
        );
        assert_eq!(
            validate_gift_amount_ngn(0),
            Err(GiftValidationError::NonPositiveAmount)
        );
    }

    #[test]
    fn open_gift_only_transitions_forward_once() {
        assert!(GiftStatus::Open.can_transition_to(GiftStatus::Claimed));
        assert!(GiftStatus::Open.can_transition_to(GiftStatus::Expired));
        assert!(GiftStatus::Open.can_transition_to(GiftStatus::Cancelled));
        assert!(!GiftStatus::Claimed.can_transition_to(GiftStatus::Expired));
        assert!(!GiftStatus::Expired.can_transition_to(GiftStatus::Claimed));
    }

    #[test]
    fn school_fees_is_not_giftable() {
        assert!(!is_giftable_utility("school_fees"));
        assert!(is_giftable_utility("airtime"));
    }

    #[test]
    fn held_gift_outcome_covers_the_claim_and_expiry_matrix() {
        let now = chrono::Utc::now();
        let past = now - chrono::Duration::minutes(1);
        let future = now + chrono::Duration::hours(1);

        assert_eq!(
            held_gift_outcome(GiftStatus::Claimed, None, now),
            HeldGiftOutcome::ReleaseToDispatch
        );
        assert_eq!(
            held_gift_outcome(GiftStatus::Expired, None, now),
            HeldGiftOutcome::ResolveExpired
        );
        assert_eq!(
            held_gift_outcome(GiftStatus::Cancelled, None, now),
            HeldGiftOutcome::FlagForReview
        );
        assert_eq!(
            held_gift_outcome(GiftStatus::Open, None, now),
            HeldGiftOutcome::KeepHeld
        );
        assert_eq!(
            held_gift_outcome(GiftStatus::Open, Some(future), now),
            HeldGiftOutcome::KeepHeld
        );
        assert_eq!(
            held_gift_outcome(GiftStatus::Open, Some(past), now),
            HeldGiftOutcome::ResolveExpired
        );
    }
}
