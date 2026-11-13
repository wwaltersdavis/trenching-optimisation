pub mod csv_output;
pub mod detection_stats;

pub use csv_output::write_to_csv;
pub use detection_stats::*;

use crate::Percentage;

pub fn mean<T>(values: &[T]) -> T
where
    T: Copy + std::iter::Sum<T> + std::ops::Div<Output = T> + From<u32>,
{
    let sum: T = values.iter().copied().sum();
    let count = T::from(values.len() as u32);
    sum / count
}

pub fn standard_deviation<T>(values: &[T]) -> f64
where
    T: Copy + Into<f64>,
{
    let count = values.len();
    if count < 2 {
        return f64::NAN;
    }

    let floats: Vec<f64> = values.iter().map(|&v| v.into()).collect();
    let mean_value = mean(&floats);
    let variance = floats
        .iter()
        .map(|&value| (value - mean_value).powi(2))
        .sum::<f64>()
        / (count - 1) as f64;

    variance.sqrt()
}

pub fn percentile(percentages: &[Percentage], percentile: f64) -> Percentage {
    if percentages.is_empty() {
        panic!("No percentages found for percentile calculation");
    }

    let mut percentages = percentages.to_vec();
    percentages.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if percentages.len() == 1 {
        return percentages[0];
    }

    let rank = percentile.clamp(0.0, 1.0) * (percentages.len() - 1) as f64;
    let lower_index = rank.floor() as usize;
    let upper_index = rank.ceil() as usize;

    if lower_index == upper_index {
        percentages[lower_index]
    } else {
        let weight = rank - lower_index as f64;
        percentages[lower_index] * (1.0 - weight) + percentages[upper_index] * weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(mean(&values), 3.0);

        let percentages: Vec<Percentage> = values.iter().map(|&v| Percentage(v)).collect();
        assert_eq!(mean(&percentages).0, 3.0);
    }

    #[test]
    fn test_standard_deviation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((standard_deviation(&values) - 1.5811388300841898).abs() < 1e-10);
        assert!(standard_deviation(&[1.0]).is_nan());

        let percentages: Vec<Percentage> = values.iter().map(|&v| Percentage(v)).collect();
        assert!((standard_deviation(&percentages) - 1.5811388300841898).abs() < 1e-10);
    }

    #[test]
    fn test_percentile() {
        let values: Vec<Percentage> = vec![1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .map(Percentage)
            .collect();
        assert_eq!(percentile(&values, 0.0), Percentage(1.0));
        assert_eq!(percentile(&values, 0.25), Percentage(2.0));
        assert_eq!(percentile(&values, 0.5), Percentage(3.0));
        assert_eq!(percentile(&values, 0.75), Percentage(4.0));
        assert_eq!(percentile(&values, 1.0), Percentage(5.0));
        assert_eq!(percentile(&values, 0.6), Percentage(3.4));
        assert_eq!(percentile(&values, -0.1), Percentage(1.0)); // Clamped to 0.0
        assert_eq!(percentile(&values, 1.1), Percentage(5.0)); // Clamped to 1.0
    }
}
