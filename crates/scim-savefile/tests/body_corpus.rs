//! Integration test: decompress the body of a real `.sav` fixture.

use std::path::PathBuf;
use scim_savefile::{read_header, read_body};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join(name)
}

#[test]
fn creative_test_sav_body_decompresses() {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav"))
        .expect("missing fixture: tests/corpus/CREATIVE TEST.sav");

    let (header, consumed) = read_header(&bytes).expect("header should parse");
    let body = read_body(&bytes[consumed..], header.save_version)
        .expect("body should decompress");

    // Spec invariants:
    assert!(!body.is_empty(), "decompressed body should not be empty");
    assert!(body.len() > bytes.len() - consumed,
        "decompressed body should be larger than the compressed body");

    // The decompressed body starts with a u64 length prefix (for save_version >= 41).
    // We don't enforce an exact value here — that's P1.2-b's job — but sanity-check
    // it is a plausible number (positive, < body.len()).
    assert!(body.len() >= 8, "body too short to contain a length prefix");
    let prefix = u64::from_le_bytes(body[..8].try_into().unwrap());
    let body_len_u64 = u64::try_from(body.len()).expect("body.len() fits u64");
    assert!(prefix > 0 && prefix < body_len_u64,
        "body length prefix should be plausible: prefix={prefix} body.len()={}", body.len());

    let compressed_body_size = bytes.len() - consumed;
    // Use lossless integer arithmetic for the ratio computation; cast only at the
    // final display step where precision loss is acceptable.
    #[allow(clippy::cast_precision_loss)] // display-only ratio, integer precision not required
    let ratio = body.len() as f64 / compressed_body_size as f64;
    eprintln!(
        "CREATIVE TEST.sav: compressed body = {compressed_body_size} bytes, decompressed = {} bytes ({ratio:.2}x expansion)",
        body.len(),
    );
}
