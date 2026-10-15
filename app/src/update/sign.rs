//! V105-20 Update signatures: the installer must carry a minisign signature made with the
//! author's key, whose public half is built into the app (`app/update.pub`). A release
//! without a valid `.sig`, or with one from another key, is deleted and never started.

use std::fs;
use std::path::Path;

use minisign_verify::{PublicKey, Signature};

const NO_MATCH: &str = "The file didn't match, so it was deleted.";
const NOT_SIGNED: &str = "This update isn't signed, so it wasn't installed.";

/// Tauri's signer writes the key as base64 of the whole `.pub` text.
const KEY: &str = include_str!("../../update.pub");

pub const MISSING: &str = NOT_SIGNED;

/// The built-in key (None only if the file in the repo is broken; a test guards that).
pub fn embedded_key() -> Option<PublicKey> {
    key_from(KEY)
}

pub fn key_from(b64: &str) -> Option<PublicKey> {
    let text = String::from_utf8(b64_decode(b64)?).ok()?;
    PublicKey::decode(&text).ok()
}

/// Checks the file against the `.sig` text (base64 of the signature text, as `tauri signer
/// sign` writes it) with `key`. No `.sig` (None) means unsigned. Anything that fails deletes
/// the file.
pub fn verify_with(key: Option<&PublicKey>, path: &Path, sig: Option<&str>) -> Result<(), String> {
    let result = check(key, path, sig);
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}

/// The same with the built-in key.
pub fn verify(path: &Path, sig: Option<&str>) -> Result<(), String> {
    verify_with(embedded_key().as_ref(), path, sig)
}

fn check(key: Option<&PublicKey>, path: &Path, sig: Option<&str>) -> Result<(), String> {
    let sig = sig.ok_or(NOT_SIGNED)?;
    let key = key.ok_or(NO_MATCH)?;
    let text = b64_decode(sig)
        .and_then(|b| String::from_utf8(b).ok())
        .ok_or(NO_MATCH)?;
    let sig = Signature::decode(&text).map_err(|_| NO_MATCH)?;
    let bytes = fs::read(path).map_err(|_| NO_MATCH)?;
    key.verify(&bytes, &sig, false).map_err(|_| NO_MATCH.to_owned())
}

/// Standard base64 with padding; whitespace is ignored. A few lines, so no crate.
fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits, mut pad) = (0u32, 0u32, 0usize);
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace()) {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => {
                pad += 1;
                continue;
            }
            _ => return None,
        };
        if pad > 0 {
            return None;
        }
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    (pad <= 2).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test-only key pair and payload (the private half was thrown away).
    const TEST_PUB: &str = include_str!("../../tests/fixtures/sign/test-only.pub");
    const SIG: &str = include_str!("../../tests/fixtures/sign/test-only-payload.bin.sig");
    const PAYLOAD: &[u8] = include_bytes!("../../tests/fixtures/sign/test-only-payload.bin");

    fn file(tag: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("cyex-sign-test-{tag}-{}.bin", std::process::id()));
        fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn embedded_key_decodes_with_our_key_id() {
        let key = embedded_key().expect("app/update.pub decodes");
        assert!(key.untrusted_comment().unwrap().contains("A68EFB41798A41E4"));
    }

    #[test]
    fn good_signature_passes_and_keeps_the_file() {
        let key = key_from(TEST_PUB).unwrap();
        let p = file("good", PAYLOAD);
        assert_eq!(verify_with(Some(&key), &p, Some(SIG)), Ok(()));
        assert!(p.exists());
        let _ = fs::remove_file(p);
    }

    #[test]
    fn a_changed_byte_fails_and_deletes() {
        let key = key_from(TEST_PUB).unwrap();
        let mut bytes = PAYLOAD.to_vec();
        bytes[0] ^= 1;
        let p = file("flip", &bytes);
        assert_eq!(verify_with(Some(&key), &p, Some(SIG)), Err(NO_MATCH.to_owned()));
        assert!(!p.exists());
    }

    #[test]
    fn a_signature_from_another_key_fails_against_the_real_key() {
        let p = file("other", PAYLOAD);
        assert_eq!(verify(&p, Some(SIG)), Err(NO_MATCH.to_owned()));
        assert!(!p.exists());
    }

    #[test]
    fn a_missing_sig_is_unsigned() {
        let key = key_from(TEST_PUB).unwrap();
        let p = file("none", PAYLOAD);
        assert_eq!(verify_with(Some(&key), &p, None), Err(NOT_SIGNED.to_owned()));
        assert!(!p.exists());
    }

    #[test]
    fn bad_base64_or_text_fails() {
        let key = key_from(TEST_PUB).unwrap();
        for junk in ["", "%%%not base64%%%", "bm90IGEgc2lnbmF0dXJl", "<html>404</html>"] {
            let p = file("junk", PAYLOAD);
            assert_eq!(
                verify_with(Some(&key), &p, Some(junk)),
                Err(NO_MATCH.to_owned()),
                "{junk}"
            );
            assert!(!p.exists());
        }
    }

    #[test]
    fn base64_decoder() {
        assert_eq!(b64_decode("YWJj").unwrap(), b"abc");
        assert_eq!(b64_decode("YWI=\n").unwrap(), b"ab");
        assert_eq!(b64_decode("YQ==").unwrap(), b"a");
        assert!(b64_decode("YQ==Y").is_none());
        assert!(b64_decode("Y!").is_none());
    }
}
