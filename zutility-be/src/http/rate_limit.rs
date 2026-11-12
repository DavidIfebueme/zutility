use std::net::IpAddr;

use axum::http::Request;
use tower_governor::{
    errors::GovernorError,
    key_extractor::{KeyExtractor, PeerIpKeyExtractor},
};

#[derive(Clone, Copy, Debug)]
pub struct ClientIpKeyExtractor;

impl KeyExtractor for ClientIpKeyExtractor {
    type Key = IpAddr;

    fn extract<T>(&self, request: &Request<T>) -> Result<Self::Key, GovernorError> {
        if let Some(ip) = request
            .headers()
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<IpAddr>().ok())
        {
            return Ok(ip);
        }

        PeerIpKeyExtractor.extract(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, Request};

    fn request_with_real_ip(value: &'static str) -> Request<()> {
        let mut request = Request::new(());
        request
            .headers_mut()
            .insert("x-real-ip", HeaderValue::from_static(value));
        request
    }

    #[test]
    fn prefers_x_real_ip() {
        let request = request_with_real_ip("203.0.113.7");
        assert_eq!(
            ClientIpKeyExtractor.extract(&request).ok(),
            Some(IpAddr::from([203, 0, 113, 7]))
        );
    }

    #[test]
    fn ignores_invalid_x_real_ip() {
        let request = request_with_real_ip("not-an-ip");
        assert!(ClientIpKeyExtractor.extract(&request).is_err());
    }
}
