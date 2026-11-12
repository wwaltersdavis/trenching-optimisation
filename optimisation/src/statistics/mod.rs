pub mod csv_output;
// pub mod detection_stats;
pub mod tested_feature;

pub use csv_output::write_to_csv;
// pub use detection_stats::DetectionStats;
pub use tested_feature::*;

pub fn percentage(numerator: f64, denominator: f64) -> f64 {
    if denominator == 0.0 {
        return f64::NAN;
    }
    numerator / denominator * 100.0
}

pub fn two_dp(value: f64) -> String {
    format!("{:.2}", value)
}

pub fn four_dp(value: f64) -> String {
    format!("{:.4}", value)
}

// TODO: rename this function to mean_from_sum_and_count or something
// similar, as it is not a generic mean function.
pub fn mean(values: &[f64]) -> f64 {
    let sum: f64 = values.iter().sum();
    let count = values.len();
    sum / count as f64
}

pub fn standard_deviation(values: &[f64]) -> f64 {
    let count = values.len();
    if count < 2 {
        return f64::NAN;
    }

    let mean_value = mean(values);
    let variance = values
        .iter()
        .map(|&value| (value - mean_value).powi(2))
        .sum::<f64>()
        / (count - 1) as f64;

    variance.sqrt()
}

pub fn percentile(values: &[f64], percentile: f64) -> f64 {
    let mut clean_values: Vec<f64> = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();

    if clean_values.is_empty() {
        return f64::NAN;
    }

    clean_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if clean_values.len() == 1 {
        return clean_values[0];
    }

    let rank = percentile.clamp(0.0, 1.0) * (clean_values.len() - 1) as f64;
    let lower_index = rank.floor() as usize;
    let upper_index = rank.ceil() as usize;

    if lower_index == upper_index {
        clean_values[lower_index]
    } else {
        let weight = rank - lower_index as f64;
        clean_values[lower_index] * (1.0 - weight) + clean_values[upper_index] * weight
    }
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;

    #[test]
    fn test_percentage() {
        assert_eq!(percentage(50.0, 200.0), 25.0);
    }

    // #[test]
    // fn test_mean() {
    //     assert_eq!(mean(100.0, 4), 25.0);
    //     assert!(mean(100.0, 0).is_nan());
    // }

    #[test]
    fn test_two_dp() {
        assert_eq!(two_dp(3.589), "3.59");
        assert_eq!(two_dp(f64::NAN), "NaN");
    }
}
