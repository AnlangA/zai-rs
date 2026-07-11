//! Small SHA-256 helper used for contract blob verification.

use sha2::{Digest, Sha256};

/// Hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    // Manual hex encoding avoids pulling in another dependency and keeps the
    // output lower-case (matching `shasum -a 256`).
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}
