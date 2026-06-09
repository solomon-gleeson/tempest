use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use rand::RngCore;
use std::path::PathBuf;

const KEY_FILE: &str = "vortex.key";
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

fn key_path() -> PathBuf {
    crate::config::Config::config_dir().join(KEY_FILE)
}

fn load_or_create_key() -> [u8; KEY_LEN] {
    let path = key_path();
    if path.exists() {
        let mut key = [0u8; KEY_LEN];
        let raw = std::fs::read(&path).unwrap_or_default();
        if raw.len() == KEY_LEN {
            key.copy_from_slice(&raw);
            return key;
        }
    }

    let mut key = [0u8; KEY_LEN];
    OsRng.fill_bytes(&mut key);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    use std::os::unix::fs::PermissionsExt;
    std::fs::write(&path, key).ok();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).ok();

    key
}

pub fn encrypt(plaintext: &str) -> String {
    let key = load_or_create_key();
    let cipher = Aes256Gcm::new_from_slice(&key).expect("valid key");

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("encryption failed");

    let mut combined = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    base64::engine::general_purpose::STANDARD.encode(&combined)
}

pub fn decrypt(encoded: &str) -> Option<String> {
    let key = load_or_create_key();
    let cipher = Aes256Gcm::new_from_slice(&key).expect("valid key");

    let combined = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;

    if combined.len() < NONCE_LEN {
        return None;
    }

    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}
