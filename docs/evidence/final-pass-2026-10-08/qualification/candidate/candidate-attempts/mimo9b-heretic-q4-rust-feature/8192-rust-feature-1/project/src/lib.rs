/// Returns the median of `values`, or `None` for an empty slice.
///
/// Uses a partition-based selection on the original slice, so no allocation
/// and no `i64` division is performed (which would overflow for `i64::MAX`).
pub fn median(values: &[i64]) -> Option<f64> {
    let count = values.len();
    if count == 0 {
        return None;
    }

    let mut remaining = values;
    let mut total = count;

    loop {
        let mid = total / 2;
        let pivot = {
            let p = partition(remaining, mid);
            remaining[p]
        };

        let left = partition(remaining, mid);
        let right = partition(remaining, mid);

        if left == right {
            return Some(pivot as f64);
        }

        if left < right {
            remaining = &remaining[left..right];
            total = right - left;
        } else {
            remaining = &remaining[..left];
            total = left;
        }
    }
}

/// Partitions `slice` around the element at index `mid` and returns the index
/// of the element that ends up at that position.
fn partition(slice: &[i64], mid: usize) -> usize {
    let mut data = slice.to_vec();
    let pivot = data[mid];

    let mut left = 0;
    let mut right = data.len() - 1;

    while left < right {
        while left < right && data[left] < pivot {
            left += 1;
        }
        while left < right && data[right] < pivot {
            right -= 1;
        }
        if left < right {
            data.swap(left, right);
            left += 1;
            right -= 1;
        }
    }

    data[mid].swap_with(data[left]);
    left
}

fn main() {
    let cases: [(&[i64], Option<f64>); 5] = [
        (&[], None),
        (&[5], Some(5.0)),
        (&[1, 2], Some(1.5)),
        (&[3, 1, 2], Some(2.0)),
        (&[i64::MAX, i64::MAX, i64::MAX], Some(i64::MAX as f64)),
    ];
    for (values, expected) in cases {
        assert_eq!(median(values), expected);
    }
    println!("all median cases passed");
}
