use crate::domain::phone::{PhoneNumber, requires_phone_number};

use super::error::ApiError;

pub(crate) fn normalize(utility_type: &str, raw: &str) -> Result<String, ApiError> {
    if !requires_phone_number(utility_type) {
        return Ok(raw.trim().to_owned());
    }

    PhoneNumber::parse(raw)
        .map(|number| number.as_str().to_owned())
        .map_err(|error| ApiError::bad_request(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_a_spaced_phone_number_to_the_local_form() {
        assert_eq!(
            normalize("airtime", "0706 578 0853").ok().as_deref(),
            Some("07065780853")
        );
    }

    #[test]
    fn rejects_an_invalid_phone_number_before_it_reaches_a_provider() {
        assert!(matches!(
            normalize("data", "0706 578 085"),
            Err(ApiError::BadRequest(_))
        ));
    }

    #[test]
    fn leaves_non_phone_references_alone() {
        assert_eq!(
            normalize("electricity", " 5401 2345 678 ").ok().as_deref(),
            Some("5401 2345 678")
        );
    }
}
