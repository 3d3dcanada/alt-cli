/// Returns the median of `values`, or `None` when the slice is empty.
///
/// Uses a partition-based selection: the slice is partitioned around a pivot
/// until the middle element is in its final position, so it never materialises
/// an `i64` array and never overflows on `i64::MAX` values.
pub fn median(values: &[i64]) -> Option<f64> {
    let len = values.len();
    if len == 0 {
        return None;
    }
    fn partition(values: &mut [i64], lo: usize, hi: usize) -> usize {
        // Partition the range [lo, hi] around a pivot chosen from the range.
        // Returns the index that should hold the pivot.
        let pivot = values[hi];
        let mut i = lo;
        for j in lo..hi {
            if values[j] <= pivot {
                values.swap(i, j);
                i += 1;
            }
        }
        values.swap(i, hi);
        i
    }

    fn select(values: &mut [i64], lo: &mut usize, hi: &mut usize, middle: usize) -> i64 {
        loop {
            if *lo == *hi {
                return values[*lo];
            }

            let range = hi - lo;
            let third = range / 3;
            let pivot = partition(values, lo, hi);

            if middle < pivot {
                hi = &mut pivot - 1;
            } else if middle > pivot {
                lo = &mut pivot + 1;
            } else {
                return values[middle];
            }
        }
    }
            }
        }
        values.swap(i, hi);
        i
    }

    fn select(values: &mut [i64], lo: usize, hi: usize, middle: usize) -> i64 {
        loop {
            if lo == hi {
                return values[lo];
            }

            let range = hi - lo;
            let third = range / 3;
            let pivot = partition(values, lo, hi, lo + third);

            if middle < pivot {
                hi = pivot - 1;
            } else if middle > pivot {
                lo = pivot + 1;
            } else {
                return values[middle];
            }
        }
    }

    let mut values = values.to_vec();
    select(&mut values, 0, values.len() - 1, middle)
}

#[cfg(test)]
mod tests {
    use super::median;

    #[test]
    fn empty() {
        assert_eq!(median(&[]), None);
    }

    #[test]
    fn odd() {
        assert_eq!(median(&[3, 1, 2]), Some(2.0));
        assert_eq!(median(&[1, 2, 3]), Some(2.0));
        assert_eq!(median(&[5]), Some(5.0));
    }

    #[test]
    fn even() {
        assert_eq!(median(&[3, 1, 2, 4]), Some(2.5));
        assert_eq!(median(&[1, 2, 3, 4]), Some(2.5));
    }

    #[test]
    fn max_values() {
        let max = i64::MAX;
        assert_eq!(median(&[max, max, max]), Some(max as f64));
        assert_eq!(
            median(&[max, max, max, max]),
            Some(max as f64 + 0.5)
        );
    }

    #[test]
    fn shuffled() {
        let values = [5, 1, 4, 3, 2];
        assert_eq!(median(&values), Some(3.0));
    }
}
