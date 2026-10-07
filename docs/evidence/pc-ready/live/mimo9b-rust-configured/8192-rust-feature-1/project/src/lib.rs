pub fn median(values: &[i64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    let mut sorted = values.to_vec();
    sorted.sort();

    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        // Use integer division to match the expected -0.5 result.
        Some(((sorted[mid - 1] as f64) + (sorted[mid] as f64)) / 2.0)
    } else {
        Some(sorted[mid] as f64)
    }
}
