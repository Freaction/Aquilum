pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() { 0.0 } else { values.iter().sum::<f64>() / values.len() as f64 }
}

pub fn percentiles<const N: usize>(values: &[f64], quantiles: [f64; N]) -> [f64; N] {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    quantiles.map(|q| sorted.get(((sorted.len().max(1) - 1) as f64 * q).round() as usize).copied().unwrap_or(0.0))
}

#[cfg(any(test, feature = "memory-probe"))]
pub fn mb(bytes: usize) -> f64 {
    bytes as f64 / 1_048_576.0
}
