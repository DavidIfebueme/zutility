use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneNumber(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PhoneNumberError {
    #[error("enter a valid Nigerian mobile number, like 08012345678")]
    Invalid,
}

impl PhoneNumber {
    pub fn parse(raw: &str) -> Result<Self, PhoneNumberError> {
        Ok(Self(raw.trim().to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn requires_phone_number(utility_type: &str) -> bool {
    matches!(utility_type, "airtime" | "data" | "waec" | "jamb")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical(raw: &str) -> Option<String> {
        PhoneNumber::parse(raw).ok().map(|number| number.as_str().to_owned())
    }

    #[test]
    fn accepts_a_number_with_spaces() {
        assert_eq!(canonical("0706 578 0853").as_deref(), Some("07065780853"));
    }

    #[test]
    fn accepts_dashes_parentheses_and_dots() {
        assert_eq!(canonical("0803-123-4567").as_deref(), Some("08031234567"));
        assert_eq!(canonical("(0803) 123.4567").as_deref(), Some("08031234567"));
    }

    #[test]
    fn accepts_international_prefixes() {
        assert_eq!(canonical("+2348031234567").as_deref(), Some("08031234567"));
        assert_eq!(canonical("+234 803 123 4567").as_deref(), Some("08031234567"));
        assert_eq!(canonical("2348031234567").as_deref(), Some("08031234567"));
    }

    #[test]
    fn accepts_a_ten_digit_number_without_the_leading_zero() {
        assert_eq!(canonical("8031234567").as_deref(), Some("08031234567"));
    }

    #[test]
    fn rejects_numbers_that_are_not_nigerian_mobiles() {
        assert_eq!(canonical(""), None);
        assert_eq!(canonical("080312345"), None);
        assert_eq!(canonical("080312345678"), None);
        assert_eq!(canonical("01234567890"), None);
        assert_eq!(canonical("0803 123 456a"), None);
    }

    #[test]
    fn phone_utilities_are_the_ones_that_dispatch_to_a_mobile_number() {
        for utility_type in ["airtime", "data", "waec", "jamb"] {
            assert!(requires_phone_number(utility_type), "{utility_type}");
        }
        for utility_type in ["electricity", "dstv", "gotv", "startimes", "school_fees"] {
            assert!(!requires_phone_number(utility_type), "{utility_type}");
        }
    }
}
