/// Returns the median of the values, or `None` for an empty slice.
///
/// Values are converted to `f64` before averaging, so `i64::MAX` values
/// never overflow.
pub fn median(values: &[i64]) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort();

    let n = sorted.len();
    if n == 0 {
        return None;
    }

    if n % 2 == 1 {
        Some(sorted[n / 2] as f64)
    } else {
        let low = sorted[n / 2 - 1] as f64;
        let high = sorted[n / 2] as f64;
        Some(low + (high - low) / 2.0)
    }
}
