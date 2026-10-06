use fixture_stats::median;
#[test] fn behavior(){assert_eq!(median(&[]),None);assert_eq!(median(&[9,1,5]),Some(5.0));assert_eq!(median(&[4,1,2,3]),Some(2.5));assert_eq!(median(&[i64::MAX,i64::MAX]),Some(i64::MAX as f64));}
