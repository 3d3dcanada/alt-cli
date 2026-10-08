// Structurally valid GGUF containing one float, not model weights or inference.
pub fn fixture(split: Option<(u16, u16)>) -> Vec<u8> {
    fn string(b: &mut Vec<u8>, s: &str) {
        b.extend_from_slice(&(s.len() as u64).to_le_bytes());
        b.extend_from_slice(s.as_bytes());
    }
    let mut b = b"GGUF".to_vec();
    b.extend_from_slice(&3u32.to_le_bytes());
    b.extend_from_slice(&1u64.to_le_bytes());
    b.extend_from_slice(&(if split.is_some() { 4u64 } else { 1u64 }).to_le_bytes());
    string(&mut b, "general.architecture");
    b.extend_from_slice(&8u32.to_le_bytes());
    string(&mut b, "fixture-no-model");
    if let Some((index, count)) = split {
        for (key, value) in [("split.no", index), ("split.count", count)] {
            string(&mut b, key);
            b.extend_from_slice(&2u32.to_le_bytes());
            b.extend_from_slice(&value.to_le_bytes());
        }
        string(&mut b, "split.tensors.count");
        b.extend_from_slice(&10u32.to_le_bytes());
        b.extend_from_slice(&(count as u64).to_le_bytes());
    }
    string(&mut b, "fixture.scalar");
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&1u64.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&0u64.to_le_bytes());
    b.resize(b.len().div_ceil(32) * 32, 0);
    b.extend_from_slice(&(split.map(|s| s.0).unwrap_or(0) as f32).to_le_bytes());
    b
}
