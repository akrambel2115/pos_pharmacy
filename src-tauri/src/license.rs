//! Signed license tokens bound to the machine fingerprint.
//!
//! Token format:  base64url(payload) "." base64url(signature)
//! Payload:       "PH1|<MACHINE_CODE>|<issued_unix>|<expiry_unix>"   (expiry 0 = forever)
//! Signature:     ECDSA P-256 / SHA-256 (raw r||s) made by tools/license/issue-license.ps1
//!
//! The check runs once per app launch (invoked from the frontend layout), never per action.

use base64::{engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD}, Engine};
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::Manager;

/// Public key (X||Y, base64) matching tools/license/license-private.key.
const LICENSE_PUBLIC_KEY_B64: &str =
    "CleKEeXFyy/fLigGmfISIkXSPULLMshQgLF2sJ3VttF9CYprFnfB52fj/v33arkI9/++IgCFJ3gZ9L12AUaQcg==";

const LICENSE_FILE: &str = "license.dat";

#[derive(Serialize, Deserialize, Default)]
struct StoredLicense {
    token: String,
    last_seen: i64,
}

#[derive(Serialize)]
pub struct LicenseStatus {
    /// "valid" | "missing" | "invalid" | "expired" | "clock"
    pub status: String,
    /// Unix expiry, 0 = forever
    pub expires_at: i64,
    pub machine_code: String,
}

// ---------------------------------------------------------------- fingerprint

#[cfg(windows)]
fn raw_machine_id() -> Option<String> {
    use winreg::{enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY}, RegKey};
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags("SOFTWARE\\Microsoft\\Cryptography", KEY_READ | KEY_WOW64_64KEY)
        .ok()?
        .get_value::<String, _>("MachineGuid")
        .ok()
}

#[cfg(not(windows))]
fn raw_machine_id() -> Option<String> {
    std::fs::read_to_string("/etc/machine-id").ok().map(|s| s.trim().to_string())
}

/// Stable, non-reversible machine code formatted XXXX-XXXX-XXXX-XXXX-XXXX.
pub fn machine_code() -> String {
    let raw = raw_machine_id().unwrap_or_else(|| "unknown-machine".to_string());
    let digest = Sha256::digest(format!("pharmapos|{}", raw.to_lowercase()).as_bytes());
    let hex: String = digest.iter().take(10).map(|b| format!("{:02X}", b)).collect();
    hex.as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join("-")
}

// ----------------------------------------------------------------- verification

enum VerifyError {
    Invalid,
    Expired,
}

/// Verifies signature, machine binding and expiry. Returns expiry (0 = forever).
fn verify_token(token: &str, now: i64) -> Result<i64, VerifyError> {
    let (p_b64, s_b64) = token.trim().split_once('.').ok_or(VerifyError::Invalid)?;
    let payload = URL_SAFE_NO_PAD.decode(p_b64).map_err(|_| VerifyError::Invalid)?;
    let sig_bytes = URL_SAFE_NO_PAD.decode(s_b64).map_err(|_| VerifyError::Invalid)?;

    let xy = STANDARD.decode(LICENSE_PUBLIC_KEY_B64).map_err(|_| VerifyError::Invalid)?;
    let mut sec1 = vec![0x04u8];
    sec1.extend_from_slice(&xy);
    let key = VerifyingKey::from_sec1_bytes(&sec1).map_err(|_| VerifyError::Invalid)?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| VerifyError::Invalid)?;
    key.verify(&payload, &sig).map_err(|_| VerifyError::Invalid)?;

    let text = String::from_utf8(payload).map_err(|_| VerifyError::Invalid)?;
    let parts: Vec<&str> = text.split('|').collect();
    if parts.len() != 4 || parts[0] != "PH1" {
        return Err(VerifyError::Invalid);
    }
    if parts[1] != machine_code() {
        return Err(VerifyError::Invalid);
    }
    let expiry: i64 = parts[3].parse().map_err(|_| VerifyError::Invalid)?;
    if expiry != 0 && now >= expiry {
        return Err(VerifyError::Expired);
    }
    Ok(expiry)
}

// ---------------------------------------------------------------------- storage

fn license_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(LICENSE_FILE))
}

fn read_stored(app: &tauri::AppHandle) -> Option<StoredLicense> {
    let raw = std::fs::read_to_string(license_path(app).ok()?).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_stored(app: &tauri::AppHandle, lic: &StoredLicense) -> Result<(), String> {
    let json = serde_json::to_string(lic).map_err(|e| e.to_string())?;
    std::fs::write(license_path(app)?, json).map_err(|e| e.to_string())
}

fn status(status: &str, expires_at: i64) -> LicenseStatus {
    LicenseStatus { status: status.to_string(), expires_at, machine_code: machine_code() }
}

// ------------------------------------------------------------------- commands

#[tauri::command]
pub fn get_machine_code() -> String {
    machine_code()
}

/// Called once at app launch.
#[tauri::command]
pub fn check_license(app: tauri::AppHandle) -> LicenseStatus {
    let Some(mut stored) = read_stored(&app) else {
        return status("missing", 0);
    };
    let now = chrono::Utc::now().timestamp();

    // Clock rollback protection (1 day tolerance for timezone / drift fixes)
    if stored.last_seen > now + 86_400 {
        return status("clock", 0);
    }

    match verify_token(&stored.token, now) {
        Ok(expiry) => {
            if now > stored.last_seen {
                stored.last_seen = now;
                let _ = write_stored(&app, &stored);
            }
            status("valid", expiry)
        }
        Err(VerifyError::Expired) => status("expired", 0),
        Err(VerifyError::Invalid) => status("invalid", 0),
    }
}

#[tauri::command]
pub fn activate_license(app: tauri::AppHandle, token: String) -> Result<LicenseStatus, String> {
    let now = chrono::Utc::now().timestamp();
    let token: String = token.split_whitespace().collect();
    match verify_token(&token, now) {
        Ok(expiry) => {
            write_stored(&app, &StoredLicense { token, last_seen: now })?;
            Ok(status("valid", expiry))
        }
        Err(VerifyError::Expired) => Err("expired".into()),
        Err(VerifyError::Invalid) => Err("invalid".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn e2e() {
        println!("CODE={}", machine_code());
        if let Ok(t) = std::env::var("LT") {
            let now = chrono::Utc::now().timestamp();
            assert!(verify_token(&t, now).is_ok());
            let mut bad = t.clone(); bad.insert(5, 'A');
            assert!(verify_token(&bad, now).is_err());
        }
    }
}
