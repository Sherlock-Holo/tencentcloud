use std::error::Error;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use tracing::instrument;

type HmacSha256 = Hmac<Sha256>;

#[instrument(level = "trace", err, skip(secret_key))]
pub fn calculate_authorization(
    access_id: &str,
    secret_key: &str,
    service: &str,
    host: &str,
    payload: &[u8],
    now: &OffsetDateTime,
) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
    const ALGORITHM: &str = "TC3-HMAC-SHA256";
    const CANONICAL_URI: &str = "/";
    const CANONICAL_QUERY_STRING: &str = "";
    const SIGNED_HEADERS: &str = "content-type;host";
    const FORMAT: &[FormatItem] = format_description!("[year]-[month]-[day]");

    let canonical_headers = format!("content-type:application/json; charset=utf-8\nhost:{host}\n");
    let hashed_request_payload = hex::encode(Sha256::digest(payload));

    let canonical_request = format!(
        "POST\n{CANONICAL_URI}\n{CANONICAL_QUERY_STRING}\n{canonical_headers}\n{SIGNED_HEADERS}\n{hashed_request_payload}"
    );

    let date = now.format(FORMAT)?;
    let credential_scope = format!("{date}/{service}/tc3_request");
    let hashed_canonical_request = hex::encode(Sha256::digest(canonical_request));

    let timestamp = now.unix_timestamp();
    let string_to_sign =
        format!("{ALGORITHM}\n{timestamp}\n{credential_scope}\n{hashed_canonical_request}");

    let key = format!("TC3{secret_key}");
    let secret_date = hmac_sha256(date.as_bytes(), key.as_bytes())?;
    let secret_service = hmac_sha256(service.as_bytes(), &secret_date)?;
    let secret_signing = hmac_sha256("tc3_request".as_bytes(), &secret_service)?;
    let signature = hmac_sha256(string_to_sign.as_bytes(), &secret_signing)?;
    let signature = hex::encode(signature);

    Ok(format!("{ALGORITHM} Credential={access_id}/{credential_scope},SignedHeaders=content-type;host,Signature={signature}"))
}

fn hmac_sha256(
    message: &[u8],
    key: &[u8],
) -> Result<[u8; 32], Box<dyn Error + Send + Sync + 'static>> {
    let mut hmac_sha256 = HmacSha256::new_from_slice(key)?;
    hmac_sha256.update(message);

    Ok(hmac_sha256.finalize().into_bytes().try_into().unwrap())
}
