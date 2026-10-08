//! Bounded GGUF structure/metadata inspection. Tensor weights are never allocated.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub version: u32,
    pub architecture: Option<String>,
    pub native_context: Option<u64>,
    pub blocks: Option<u64>,
    pub embedding: Option<u64>,
    pub attention_heads: Option<u64>,
    pub kv_heads: Option<u64>,
    pub key_length: Option<u64>,
    pub value_length: Option<u64>,
    pub file_type: Option<u64>,
    pub tensors: u64,
    pub split_index: Option<u64>,
    pub split_count: Option<u64>,
    pub total_tensors: Option<u64>,
    pub tensor_payload_sizes_verified: bool,
}
struct Reader {
    file: BufReader<File>,
    length: u64,
    scanned: u64,
}
impl Reader {
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        self.scanned = self
            .scanned
            .checked_add(N as u64)
            .context("GGUF size overflow")?;
        ensure!(
            self.scanned <= 64 * 1024 * 1024,
            "GGUF metadata exceeds 64 MiB inspection limit"
        );
        let mut b = [0; N];
        self.file
            .read_exact(&mut b)
            .context("Truncated GGUF header or metadata")?;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes()?))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.bytes()?))
    }
    fn skip(&mut self, n: u64) -> Result<()> {
        self.scanned = self
            .scanned
            .checked_add(n)
            .context("GGUF metadata overflow")?;
        ensure!(
            self.scanned <= 64 * 1024 * 1024,
            "GGUF metadata exceeds 64 MiB inspection limit"
        );
        let end = self
            .file
            .stream_position()?
            .checked_add(n)
            .context("GGUF offset overflow")?;
        ensure!(end <= self.length, "Truncated GGUF metadata");
        self.file.seek(SeekFrom::Start(end))?;
        Ok(())
    }
    fn string(&mut self, retain: bool) -> Result<Option<String>> {
        let n = self.u64()?;
        ensure!(
            n <= 16 * 1024 * 1024,
            "GGUF string exceeds inspection limit"
        );
        if !retain {
            self.skip(n)?;
            return Ok(None);
        }
        ensure!(n <= 4096, "GGUF metadata name exceeds 4096 bytes");
        let mut bytes = vec![0; n as usize];
        self.scanned = self
            .scanned
            .checked_add(n)
            .context("GGUF metadata overflow")?;
        ensure!(
            self.scanned <= 64 * 1024 * 1024,
            "GGUF metadata exceeds 64 MiB inspection limit"
        );
        self.file
            .read_exact(&mut bytes)
            .context("Truncated GGUF string")?;
        Ok(Some(
            String::from_utf8(bytes).context("GGUF metadata is not UTF-8")?,
        ))
    }
    fn value(
        &mut self,
        kind: u32,
        retain: bool,
        depth: usize,
    ) -> Result<Option<serde_json::Value>> {
        use serde_json::json;
        ensure!(depth <= 1, "Nested GGUF arrays are invalid");
        Ok(match kind {
            0 => Some(json!(self.bytes::<1>()?[0])),
            1 => Some(json!(i8::from_le_bytes(self.bytes()?))),
            2 => Some(json!(u16::from_le_bytes(self.bytes()?))),
            3 => Some(json!(i16::from_le_bytes(self.bytes()?))),
            4 => Some(json!(self.u32()?)),
            5 => Some(json!(i32::from_le_bytes(self.bytes()?))),
            6 => {
                self.skip(4)?;
                None
            }
            7 => {
                let n = self.bytes::<1>()?[0];
                ensure!(n <= 1, "Invalid GGUF boolean");
                Some(json!(n == 1))
            }
            8 => self.string(retain)?.map(|s| json!(s)),
            9 => {
                let subtype = self.u32()?;
                ensure!(subtype != 9, "Nested GGUF arrays are invalid");
                let count = self.u64()?;
                ensure!(
                    count <= 2_000_000,
                    "GGUF array count exceeds inspection limit"
                );
                for _ in 0..count {
                    self.value(subtype, false, depth + 1)?;
                }
                None
            }
            10 => Some(json!(self.u64()?)),
            11 => Some(json!(i64::from_le_bytes(self.bytes()?))),
            12 => {
                self.skip(8)?;
                None
            }
            _ => bail!("Unknown GGUF metadata value type {kind}"),
        })
    }
}
fn tensor_size(kind: u32, dimensions: &[u64]) -> Result<Option<u64>> {
    let (block, bytes) = match kind {
        0 => (1, 4),
        1 => (1, 2),
        2 => (32, 18),
        3 => (32, 20),
        6 => (32, 22),
        7 => (32, 24),
        8 => (32, 34),
        9 => (32, 40),
        10 => (256, 84),
        11 => (256, 110),
        12 => (256, 144),
        13 => (256, 176),
        14 => (256, 210),
        15 => (256, 292),
        24 => (1, 1),
        25 => (1, 2),
        26 => (1, 4),
        27 => (1, 8),
        28 => (1, 8),
        30 => (1, 2),
        _ => return Ok(None),
    };
    ensure!(
        dimensions[0].is_multiple_of(block),
        "GGUF quantized tensor row is not block aligned"
    );
    let elements = dimensions
        .iter()
        .try_fold(1u64, |n, d| n.checked_mul(*d))
        .context("GGUF tensor dimensions overflow")?;
    Ok(Some(
        (elements / block)
            .checked_mul(bytes)
            .context("GGUF tensor size overflow")?,
    ))
}
pub fn inspect(path: &Path) -> Result<Metadata> {
    let file = File::open(path).context("The model file is missing or cannot be read")?;
    let info = file.metadata()?;
    ensure!(info.is_file(), "GGUF must be a regular file");
    let mut r = Reader {
        file: BufReader::new(file),
        length: info.len(),
        scanned: 0,
    };
    ensure!(&r.bytes::<4>()? == b"GGUF", "This is not a GGUF model file");
    let version = r.u32()?;
    ensure!(
        (2..=3).contains(&version),
        "Supported GGUF versions are 2 and 3"
    );
    let tensors = r.u64()?;
    let entries = r.u64()?;
    ensure!(
        (1..=1_000_000).contains(&tensors) && entries <= 100_000,
        "Unreasonable GGUF tensor or metadata count"
    );
    let mut values = BTreeMap::new();
    let mut names = std::collections::HashSet::new();
    for _ in 0..entries {
        let key = r.string(true)?.context("GGUF key")?;
        ensure!(
            names.insert(key.clone()),
            "Duplicate GGUF metadata key {key}"
        );
        let kind = r.u32()?;
        let retain = matches!(
            key.as_str(),
            "general.architecture"
                | "general.alignment"
                | "general.file_type"
                | "split.no"
                | "split.count"
                | "split.tensors.count"
        ) || [
            ".context_length",
            ".block_count",
            ".embedding_length",
            ".attention.head_count",
            ".attention.head_count_kv",
            ".attention.key_length",
            ".attention.value_length",
        ]
        .iter()
        .any(|suffix| key.ends_with(suffix));
        if let Some(value) = r.value(kind, retain, 0)?
            && retain
        {
            values.insert(key, value);
        }
    }
    let number = |key: &str| values.get(key).and_then(serde_json::Value::as_u64);
    let alignment = number("general.alignment").unwrap_or(32);
    ensure!(
        alignment.is_power_of_two() && alignment <= 4096,
        "Invalid GGUF alignment"
    );
    let mut spans = Vec::new();
    let mut verified = true;
    let mut tensor_names = std::collections::HashSet::new();
    for _ in 0..tensors {
        let name = r.string(true)?.context("GGUF tensor name")?;
        ensure!(tensor_names.insert(name), "Duplicate GGUF tensor name");
        let count = r.u32()?;
        ensure!(
            (1..=4).contains(&count),
            "Invalid GGUF tensor dimension count"
        );
        let mut dimensions = Vec::new();
        for _ in 0..count {
            let n = r.u64()?;
            ensure!(n > 0, "Zero GGUF tensor dimension");
            dimensions.push(n);
        }
        let kind = r.u32()?;
        let offset = r.u64()?;
        ensure!(
            offset.is_multiple_of(alignment),
            "Unaligned GGUF tensor offset"
        );
        let size = tensor_size(kind, &dimensions)?;
        verified &= size.is_some();
        spans.push((offset, size.unwrap_or(1)));
    }
    let position = r.file.stream_position()?;
    let data = position
        .checked_add(alignment - 1)
        .context("GGUF offset overflow")?
        / alignment
        * alignment;
    spans.sort_unstable();
    let mut previous_end = 0;
    for (offset, size) in spans {
        ensure!(offset >= previous_end, "Overlapping GGUF tensor payloads");
        let end = offset
            .checked_add(size)
            .context("GGUF tensor offset overflow")?;
        ensure!(
            data.checked_add(end).is_some_and(|n| n <= r.length),
            "Truncated GGUF tensor payload"
        );
        previous_end = end;
    }
    let architecture = values
        .get("general.architecture")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let field = |suffix: &str| {
        architecture
            .as_ref()
            .and_then(|arch| number(&format!("{arch}.{suffix}")))
    };
    let metadata = Metadata {
        version,
        native_context: field("context_length"),
        blocks: field("block_count"),
        embedding: field("embedding_length"),
        attention_heads: field("attention.head_count"),
        kv_heads: field("attention.head_count_kv"),
        key_length: field("attention.key_length"),
        value_length: field("attention.value_length"),
        file_type: number("general.file_type"),
        tensors,
        split_index: number("split.no"),
        split_count: number("split.count"),
        total_tensors: number("split.tensors.count"),
        tensor_payload_sizes_verified: verified,
        architecture,
    };
    if let Some(count) = metadata.split_count {
        ensure!(
            (1..=1024).contains(&count) && metadata.split_index.is_some_and(|n| n < count),
            "Invalid GGUF split metadata"
        );
    }
    Ok(metadata)
}
