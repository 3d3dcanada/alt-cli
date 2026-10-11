//! B09 (audit A12): GGUF import rejects magic-only, truncated and unsupported headers.
use alt_cli::models::{gguf_header, import};
use std::sync::{Arc, atomic::AtomicBool};

fn header(version: u32, tensors: u64, metadata: u64) -> Vec<u8> {
    let mut bytes = b"GGUF".to_vec();
    bytes.extend(version.to_le_bytes());
    bytes.extend(tensors.to_le_bytes());
    bytes.extend(metadata.to_le_bytes());
    bytes
}

#[test]
fn fixed_header_rules() {
    assert!(gguf_header(&header(3, 291, 24)).is_ok());
    assert!(
        gguf_header(&header(2, 0, 3)).is_ok(),
        "a metadata-only first split is valid"
    );
    assert!(
        gguf_header(b"GGUF")
            .unwrap_err()
            .to_string()
            .contains("truncated")
    );
    assert!(gguf_header(&header(3, 1, 1)[..20]).is_err());
    assert!(
        gguf_header(&header(1, 1, 1))
            .unwrap_err()
            .to_string()
            .contains("version")
    );
    assert!(
        gguf_header(&header(3, u64::MAX, 1))
            .unwrap_err()
            .to_string()
            .contains("implausible")
    );
    assert!(gguf_header(&header(3, 1, 1)).is_ok());
}

#[tokio::test]
async fn import_refuses_a_magic_only_file_and_keeps_the_library_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("state");
    let magic_only = dir.path().join("magic.gguf");
    std::fs::write(&magic_only, b"GGUF").unwrap();
    let error = import(
        &root,
        &magic_only,
        false,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(error.contains("truncated"), "{error}");
    let valid = dir.path().join("valid.gguf");
    let mut bytes = header(3, 0, 0);
    bytes.extend(b"fixture payload, no real weights");
    std::fs::write(&valid, bytes).unwrap();
    import(
        &root,
        &valid,
        false,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(alt_cli::models::library(&root).unwrap().len(), 1);
}
