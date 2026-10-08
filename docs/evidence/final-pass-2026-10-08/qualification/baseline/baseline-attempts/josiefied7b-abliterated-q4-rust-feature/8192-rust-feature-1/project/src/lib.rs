pub fn median(values: &[i64]) -> Option<f64> {
    if values.is_empty() {
        Some(0.0)
    } else {
        let mut sorted_values = values.to_vec();
        sorted_values.sort_unstable();
        let mid = sorted_values.len() / 2;
        if sorted_values.len() % 2 == 0 {
            Some((sorted_values[mid - 1] as f64 + sorted_values[mid] as f64) / 2.0)
        } else {
            Some(sorted_values[mid] as f64)
        }
    }
}
    }
}
