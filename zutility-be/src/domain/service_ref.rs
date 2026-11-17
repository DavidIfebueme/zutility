use crate::domain::phone::{PhoneNumber, requires_phone_number};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ServiceRefError {
    #[error("enter a valid Nigerian mobile number, like 08012345678")]
    InvalidPhoneNumber,
    #[error("enter a numeric account number")]
    InvalidAccountNumber,
    #[error("enter a valid account identifier")]
    InvalidIdentifier,
    #[error("service_ref is required")]
    Empty,
}

pub fn normalize(
    country_iso2: &str,
    utility_type: &str,
    raw: &str,
) -> Result<String, ServiceRefError> {
    match country_iso2.trim().to_ascii_uppercase().as_str() {
        "NG" | "" => normalize_nigeria(utility_type, raw),
        "SN" | "GM" | "ML" | "BJ" | "BF" | "CI" | "TG" | "MW" => normalize_numeric_account(raw),
        _ => normalize_identifier(raw),
    }
}

fn normalize_nigeria(utility_type: &str, raw: &str) -> Result<String, ServiceRefError> {
    if !requires_phone_number(utility_type) {
        return non_empty(raw.trim());
    }

    PhoneNumber::parse(raw)
        .map(|number| number.as_str().to_owned())
        .map_err(|_| ServiceRefError::InvalidPhoneNumber)
}

fn normalize_numeric_account(raw: &str) -> Result<String, ServiceRefError> {
    let account: String = raw
        .chars()
        .filter(|character| !character.is_whitespace() && !matches!(character, '-' | '(' | ')'))
        .collect();

    if !(6..=20).contains(&account.len()) || !account.chars().all(|c| c.is_ascii_digit()) {
        return Err(ServiceRefError::InvalidAccountNumber);
    }

    Ok(account)
}

fn normalize_identifier(raw: &str) -> Result<String, ServiceRefError> {
    let value = raw.trim();
    if value.is_empty() || value.len() > 64 || value.chars().any(char::is_control) {
        return Err(ServiceRefError::InvalidIdentifier);
    }

    Ok(value.to_owned())
}

fn non_empty(value: &str) -> Result<String, ServiceRefError> {
    if value.is_empty() {
        return Err(ServiceRefError::Empty);
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_a_spaced_phone_number_to_the_local_form() {
        assert_eq!(
            normalize("NG", "airtime", "0706 578 0853").ok().as_deref(),
            Some("07065780853")
        );
    }

    #[test]
    fn rejects_an_invalid_phone_number_before_it_reaches_a_provider() {
        assert!(matches!(
            normalize("NG", "data", "0706 578 085"),
            Err(ServiceRefError::InvalidPhoneNumber)
        ));
    }

    #[test]
    fn leaves_non_phone_references_alone() {
        assert_eq!(
            normalize("NG", "electricity", " 5401 2345 678 ")
                .ok()
                .as_deref(),
            Some("5401 2345 678")
        );
    }

    #[test]
    fn senegal_accepts_a_numeric_account_and_strips_spacing() {
        assert_eq!(
            normalize("SN", "electricity", " 12 345 678 ")
                .ok()
                .as_deref(),
            Some("12345678")
        );
    }

    #[test]
    fn west_african_corridors_reject_a_non_numeric_account() {
        for country in ["SN", "GM", "ML", "BJ", "BF", "CI", "TG", "MW"] {
            assert_eq!(
                normalize(country, "electricity", "12AB5678"),
                Err(ServiceRefError::InvalidAccountNumber),
                "{country}"
            );
        }
    }

    #[test]
    fn zimbabwe_and_zambia_keep_the_provided_identifier() {
        assert_eq!(
            normalize("ZW", "electricity", " 0123456789 ")
                .ok()
                .as_deref(),
            Some("0123456789")
        );
        assert_eq!(
            normalize("ZM", "television", "DSTV-1234").ok().as_deref(),
            Some("DSTV-1234")
        );
        assert_eq!(
            normalize("ZM", "television", "   "),
            Err(ServiceRefError::InvalidIdentifier)
        );
    }
}
