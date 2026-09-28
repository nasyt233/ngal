// src/crypto.rs
use anyhow::{anyhow, Result};
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

/// 打包/运行时共用密钥（32 字节）。
/// 仅用于防止普通用户直接解压，不是真正的安全加密。
const KEY_BYTES: [u8; 32] = *b"ngal_2026_secret_key_change_me!!";

/// ChaCha20Poly1305 的 nonce 长度（96 bit）
const NONCE_LEN: usize = 12;

/// 加密：输出格式 `[nonce(12B)] + [ciphertext]`
pub fn encrypt(plain: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&KEY_BYTES));
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let ct = cipher
        .encrypt(&nonce, plain)
        .map_err(|e| anyhow!("加密失败: {e}"))?;

    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(nonce.as_slice());
    out.extend_from_slice(&ct);
    Ok(out)
}

/// 解密：输入格式 `[nonce(12B)] + [ciphertext]`
pub fn decrypt(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < NONCE_LEN {
        return Err(anyhow!("加密数据长度不足"));
    }
    let (nonce_bytes, ct) = data.split_at(NONCE_LEN);

    let cipher = ChaCha20Poly1305::new(Key::from_slice(&KEY_BYTES));
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ct)
        .map_err(|e| anyhow!("解密失败（数据被篡改或密钥不符）: {e}"))
}