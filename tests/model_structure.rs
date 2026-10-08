use alt_cli::models;
use std::sync::{Arc, atomic::AtomicBool};
#[path = "fixtures/gguf.rs"]
mod gguf_fixture;
fn cancel() -> models::Cancel {
    Arc::new(AtomicBool::new(false))
}
#[tokio::test]
async fn incomplete_headers_tensor_payloads_and_unreasonable_counts_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("model.gguf");
    for bad in [
        b"GGUF".to_vec(),
        gguf_fixture::fixture(None)[..32].to_vec(),
        {
            let mut b = gguf_fixture::fixture(None);
            b.pop();
            b
        },
        {
            let mut b = gguf_fixture::fixture(None);
            b[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
            b
        },
    ] {
        std::fs::write(&path, bad).unwrap();
        assert!(
            models::import(root.path(), &path, true, cancel(), |_| {})
                .await
                .is_err()
        );
    }
    assert!(models::library(root.path()).unwrap().is_empty());
}
#[tokio::test]
async fn local_shards_require_complete_matching_metadata_and_all_hashes() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("part-00001-of-00002.gguf");
    let second = root.path().join("part-00002-of-00002.gguf");
    std::fs::write(&first, gguf_fixture::fixture(Some((0, 2)))).unwrap();
    assert!(
        models::import(root.path(), &first, true, cancel(), |_| {})
            .await
            .is_err()
    );
    std::fs::write(&second, gguf_fixture::fixture(Some((0, 2)))).unwrap();
    assert!(
        models::import(root.path(), &first, true, cancel(), |_| {})
            .await
            .is_err()
    );
    std::fs::write(&second, gguf_fixture::fixture(Some((1, 2)))).unwrap();
    // Choosing any member discovers and registers the first member/full set.
    let artifact = models::import(root.path(), &second, true, cancel(), |_| {})
        .await
        .unwrap();
    assert_eq!(artifact.path, first);
    assert_eq!(artifact.pieces.len(), 2);
    assert_eq!(
        artifact.bytes,
        std::fs::metadata(&first).unwrap().len() + std::fs::metadata(&second).unwrap().len()
    );
    models::verify_artifact(&artifact, &cancel(), |_| {})
        .await
        .unwrap();
    let mut changed = std::fs::read(&second).unwrap();
    let n = changed.len();
    changed[n - 4..].copy_from_slice(&3f32.to_le_bytes());
    std::fs::write(&second, changed).unwrap();
    assert!(
        models::verify_artifact(&artifact, &cancel(), |_| {})
            .await
            .is_err()
    );
    models::remove(root.path(), &artifact.id, true).unwrap();
    assert!(first.exists() && second.exists());
}
#[test]
fn kv_estimates_use_gqa_context_and_quantization_without_claiming_measurement() {
    let m = models::gguf::Metadata {
        blocks: Some(32),
        embedding: Some(4096),
        attention_heads: Some(32),
        kv_heads: Some(8),
        ..Default::default()
    };
    let mut settings = alt_cli::runtime::Settings::default();
    let f16 = alt_cli::hardware::kv_bytes(&m, 4096, &settings).unwrap();
    assert_eq!(f16, 32 * 8 * 128 * 4096 * 2 * 2);
    assert_eq!(
        alt_cli::hardware::kv_bytes(&m, 8192, &settings),
        Some(2 * f16)
    );
    settings.cache_k = "q8_0".into();
    settings.cache_v = "q8_0".into();
    assert!(alt_cli::hardware::kv_bytes(&m, 4096, &settings).unwrap() < f16);
    assert!(alt_cli::hardware::kv_bytes(&Default::default(), 4096, &settings).is_none());
}
