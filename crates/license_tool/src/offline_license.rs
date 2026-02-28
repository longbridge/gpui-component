use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signer, SigningKey};
use one_core::license::{OfflineLicenseDocument, OfflineLicensePayload, PlanTier};
use rand::{RngCore, rngs::OsRng};
use std::fs;
use std::path::Path;

pub fn build_offline_license_payload(
    user_id: String,
    plan: PlanTier,
    expires_at: Option<i64>,
    device_id: Option<String>,
) -> OfflineLicensePayload {
    let issued_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    OfflineLicensePayload {
        version: OfflineLicensePayload::VERSION,
        user_id,
        plan,
        expires_at,
        issued_at,
        device_id,
    }
}

pub fn signing_key_from_base64(secret_key_base64: &str) -> Result<SigningKey> {
    let decoded = BASE64
        .decode(secret_key_base64.as_bytes())
        .map_err(|e| anyhow!("私钥解码失败: {}", e))?;

    let secret_key: [u8; 32] = decoded.try_into().map_err(|_| anyhow!("私钥长度错误"))?;

    Ok(SigningKey::from_bytes(&secret_key))
}

pub fn generate_keypair_base64() -> (String, String) {
    let mut secret_key = [0u8; 32];
    OsRng.fill_bytes(&mut secret_key);
    let signing_key = SigningKey::from_bytes(&secret_key);
    let secret_key_base64 = BASE64.encode(secret_key);
    let public_key_base64 = BASE64.encode(signing_key.verifying_key().to_bytes());
    (secret_key_base64, public_key_base64)
}

pub fn generate_offline_license_document(
    payload: &OfflineLicensePayload,
    signing_key: &SigningKey,
) -> Result<OfflineLicenseDocument> {
    let payload_bytes = serde_json::to_vec(payload)?;
    let signature = signing_key.sign(&payload_bytes);

    Ok(OfflineLicenseDocument {
        payload: BASE64.encode(payload_bytes),
        signature: BASE64.encode(signature.to_bytes()),
    })
}

pub fn write_offline_license_to_path<P: AsRef<Path>>(
    document: &OfflineLicenseDocument,
    path: P,
) -> Result<()> {
    let json = serde_json::to_string_pretty(document)?;
    fs::write(path, json)?;
    Ok(())
}
