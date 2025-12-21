//! Statistics Calculator Module
//! Handles statistical computations including descriptive stats and t-tests.

use polars::prelude::*;
use rayon::prelude::*;
use statrs::distribution::{ContinuousCDF, StudentsT};
use std::collections::HashMap;

/// Significance threshold for t-test
pub const SIGNIFICANCE_THRESHOLD: f64 = 0.05;

/// Statistics for a single group.
#[derive(Debug, Clone)]
pub struct GroupStats {
    pub group_name: String,
    pub count: usize,
    pub mean: f64,
    pub median: f64,
    pub std: f64,
    pub variance: f64,
    pub p95: f64,
    pub p05: f64,
    pub std_diff_from_control: Option<f64>,
    pub p_value: Option<f64>,
    pub is_significant: bool,
}

impl Default for GroupStats {
    fn default() -> Self {
        Self {
            group_name: String::new(),
            count: 0,
            mean: f64::NAN,
            median: f64::NAN,
            std: f64::NAN,
            variance: f64::NAN,
            p95: f64::NAN,
            p05: f64::NAN,
            std_diff_from_control: None,
            p_value: None,
            is_significant: false,
        }
    }
}

/// Statistics for a data type across all groups.
#[derive(Debug, Clone)]
pub struct DataTypeStats {
    pub data_type: String,
    pub control_group: String,
    pub group_stats: HashMap<String, GroupStats>,
}

impl DataTypeStats {
    /// Get groups ordered with control first.
    pub fn get_ordered_groups(&self) -> Vec<String> {
        let mut groups: Vec<String> = self.group_stats.keys().cloned().collect();
        groups.sort();

        // Move control group to front
        if let Some(pos) = groups.iter().position(|g| g == &self.control_group) {
            groups.remove(pos);
            groups.insert(0, self.control_group.clone());
        }

        groups
    }

    /// Check if any group has significant p-value.
    pub fn has_significant_results(&self) -> bool {
        self.group_stats
            .iter()
            .any(|(name, gs)| name != &self.control_group && gs.is_significant)
    }
}

/// Handles statistical calculations with multi-threading support.
pub struct StatsCalculator;

impl StatsCalculator {
    /// Compute descriptive statistics for an array of values.
    pub fn compute_descriptive_stats(values: &[f64]) -> GroupStats {
        let n = values.len();
        if n == 0 {
            return GroupStats::default();
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mean = values.iter().sum::<f64>() / n as f64;
        let median = if n % 2 == 0 {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        } else {
            sorted[n / 2]
        };

        let variance = if n > 1 {
            values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64
        } else {
            0.0
        };
        let std = variance.sqrt();

        let p95 = Self::percentile(&sorted, 95.0);
        let p05 = Self::percentile(&sorted, 5.0);

        GroupStats {
            group_name: String::new(),
            count: n,
            mean,
            median,
            std,
            variance,
            p95,
            p05,
            std_diff_from_control: None,
            p_value: None,
            is_significant: false,
        }
    }

    /// Calculate percentile using linear interpolation (NumPy compatible).
    fn percentile(sorted_values: &[f64], p: f64) -> f64 {
        let n = sorted_values.len();
        if n == 0 {
            return f64::NAN;
        }
        if n == 1 {
            return sorted_values[0];
        }

        let rank = (p / 100.0) * (n - 1) as f64;
        let lower = rank.floor() as usize;
        let upper = (rank.ceil() as usize).min(n - 1);
        let frac = rank - lower as f64;

        if lower == upper {
            sorted_values[lower]
        } else {
            sorted_values[lower] * (1.0 - frac) + sorted_values[upper] * frac
        }
    }

    /// Perform Welch's t-test (independent samples, unequal variance).
    pub fn perform_ttest(group_values: &[f64], control_values: &[f64]) -> (f64, bool) {
        let n1 = group_values.len() as f64;
        let n2 = control_values.len() as f64;

        if n1 < 2.0 || n2 < 2.0 {
            return (f64::NAN, false);
        }

        let mean1 = group_values.iter().sum::<f64>() / n1;
        let mean2 = control_values.iter().sum::<f64>() / n2;

        let var1 = group_values
            .iter()
            .map(|x| (x - mean1).powi(2))
            .sum::<f64>()
            / (n1 - 1.0);
        let var2 = control_values
            .iter()
            .map(|x| (x - mean2).powi(2))
            .sum::<f64>()
            / (n2 - 1.0);

        let se = (var1 / n1 + var2 / n2).sqrt();
        if se == 0.0 {
            return (1.0, false); // No variance difference
        }

        let t = (mean1 - mean2) / se;

        // Welch-Satterthwaite degrees of freedom
        let df_num = (var1 / n1 + var2 / n2).powi(2);
        let df_denom = (var1 / n1).powi(2) / (n1 - 1.0) + (var2 / n2).powi(2) / (n2 - 1.0);
        let df = df_num / df_denom;

        // Two-tailed p-value using t-distribution
        if let Ok(dist) = StudentsT::new(0.0, 1.0, df) {
            let p_value = 2.0 * (1.0 - dist.cdf(t.abs()));
            let is_significant = p_value <= SIGNIFICANCE_THRESHOLD;
            (p_value, is_significant)
        } else {
            (f64::NAN, false)
        }
    }

    /// Get values for a specific group from DataFrame.
    pub fn get_values_for_group(df: &DataFrame, group: &str) -> Vec<f64> {
        df.clone()
            .lazy()
            .filter(col("group").eq(lit(group)))
            .select([col("value")])
            .collect()
            .ok()
            .and_then(|df| df.column("value").ok().cloned())
            .map(|col| {
                col.f64()
                    .ok()
                    .map(|ca| ca.into_iter().filter_map(|v| v).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Get values for a specific data_type AND group from DataFrame.
    /// This ensures the quantile plot data matches the statistics table.
    pub fn get_values_for_data_type_and_group(
        df: &DataFrame,
        data_type: &str,
        group: &str,
    ) -> Vec<f64> {
        df.clone()
            .lazy()
            .filter(
                col("data_type")
                    .eq(lit(data_type))
                    .and(col("group").eq(lit(group))),
            )
            .select([col("value")])
            .collect()
            .ok()
            .and_then(|df| df.column("value").ok().cloned())
            .map(|col| {
                col.f64()
                    .ok()
                    .map(|ca| ca.into_iter().filter_map(|v| v).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Compute statistics for all groups within a data type.
    pub fn compute_data_type_stats(
        df: &DataFrame,
        data_type: &str,
        control_group: &str,
    ) -> DataTypeStats {
        // Filter for this data type
        let type_df = df
            .clone()
            .lazy()
            .filter(col("data_type").eq(lit(data_type)))
            .collect()
            .unwrap_or_default();

        let groups: Vec<String> = type_df
            .column("group")
            .ok()
            .and_then(|col| col.unique().ok())
            .map(|unique| {
                unique
                    .as_materialized_series()
                    .iter()
                    .filter_map(|v| {
                        if v.is_null() {
                            None
                        } else {
                            Some(v.to_string().trim_matches('"').to_string())
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut group_stats: HashMap<String, GroupStats> = HashMap::new();

        // First compute control group stats
        let control_values = Self::get_values_for_group(&type_df, control_group);
        let mut control_stats = Self::compute_descriptive_stats(&control_values);
        control_stats.group_name = control_group.to_string();
        let control_std = control_stats.std;
        let control_mean = control_stats.mean;
        group_stats.insert(control_group.to_string(), control_stats);

        // Compute stats for other groups
        for group_name in &groups {
            if group_name == control_group {
                continue;
            }

            let values = Self::get_values_for_group(&type_df, group_name);
            let mut gs = Self::compute_descriptive_stats(&values);
            gs.group_name = group_name.clone();

            // Calculate standardized mean difference
            if control_std > 0.0 && !control_mean.is_nan() {
                gs.std_diff_from_control = Some((gs.mean - control_mean) / control_std);
            }

            // Perform t-test
            if !control_values.is_empty() {
                let (p_value, is_significant) = Self::perform_ttest(&values, &control_values);
                gs.p_value = Some(p_value);
                gs.is_significant = is_significant;
            }

            group_stats.insert(group_name.clone(), gs);
        }

        DataTypeStats {
            data_type: data_type.to_string(),
            control_group: control_group.to_string(),
            group_stats,
        }
    }

    /// Compute statistics for all data types in parallel.
    pub fn compute_all_stats_parallel(
        df: &DataFrame,
        control_group: &str,
    ) -> HashMap<String, DataTypeStats> {
        let data_types: Vec<String> = df
            .column("data_type")
            .ok()
            .and_then(|col| col.unique().ok())
            .map(|unique| {
                unique
                    .as_materialized_series()
                    .iter()
                    .filter_map(|v| {
                        if v.is_null() {
                            None
                        } else {
                            Some(v.to_string().trim_matches('"').to_string())
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Use rayon for parallel computation
        data_types
            .par_iter()
            .map(|data_type| {
                let stats = Self::compute_data_type_stats(df, data_type, control_group);
                (data_type.clone(), stats)
            })
            .collect()
    }
}
