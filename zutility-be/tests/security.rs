use secrecy::SecretString;
use zutility_be::http::auth;

#[test]
fn hash_ip_is_deterministic_and_secret_bound() {
    let secret_a = SecretString::from(String::from("ip-secret-a"));
    let secret_b = SecretString::from(String::from("ip-secret-b"));
    let ip = "203.0.113.9";

    let hash_a_one = auth::hash_ip(&secret_a, ip).expect("hash ip once");
    let hash_a_two = auth::hash_ip(&secret_a, ip).expect("hash ip twice");
    let hash_b = auth::hash_ip(&secret_b, ip).expect("hash ip with other secret");

    assert_eq!(hash_a_one, hash_a_two);
    assert_ne!(hash_a_one, hash_b);
}
