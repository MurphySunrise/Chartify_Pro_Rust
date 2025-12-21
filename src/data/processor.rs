//! Data Processor Module
//! Handles data cleaning and transformation (stack operation).

use polars::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessorError {
    #[error("Polars error: {0}")]
    PolarsError(#[from] PolarsError),
    #[error("Single mode requires data_type_col and value_col")]
    MissingSingleModeColumns,
    #[error("Multi mode requires data_cols")]
    MissingMultiModeColumns,
}

/// Data processing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataMode {
    /// Single column mode: data already in long format
    Single,
    /// Multi column mode: need to unpivot/melt
    Multi,
}

impl Default for DataMode {
    fn default() -> Self {
        DataMode::Single
    }
}

/// Handles data cleaning and transformation operations.
pub struct DataProcessor;

impl DataProcessor {
    /// Transform multi-column data to long format (stack operation).
    ///
    /// Output columns: [group, "data_type", "value"]
    pub fn stack_to_long(
        df: &DataFrame,
        group_col: &str,
        data_cols: &[String],
    ) -> Result<DataFrame, ProcessorError> {
        // Build the result manually by iterating through columns
        let mut groups: Vec<String> = Vec::new();
        let mut data_types: Vec<String> = Vec::new();
        let mut values: Vec<f64> = Vec::new();

        let group_series = df.column(group_col)?;

        for data_col in data_cols {
            if let Ok(value_series) = df.column(data_col) {
                let value_f64 = value_series.cast(&DataType::Float64)?;
                let value_ca = value_f64.f64()?;

                for i in 0..df.height() {
                    if let (Ok(g), Some(v)) = (group_series.get(i), value_ca.get(i)) {
                        if !v.is_nan() && !g.is_null() {
                            groups.push(g.to_string().trim_matches('"').to_string());
                            data_types.push(data_col.clone());
                            values.push(v);
                        }
                    }
                }
            }
        }

        let df = DataFrame::new(vec![
            Column::new("group".into(), groups),
            Column::new("data_type".into(), data_types),
            Column::new("value".into(), values),
        ])?;

        Ok(df)
    }

    /// Prepare data based on mode (single or multi-column).
    ///
    /// Output format: ["group", "data_type", "value"]
    pub fn prepare_data(
        df: &DataFrame,
        mode: DataMode,
        group_col: &str,
        data_type_col: Option<&str>,
        value_col: Option<&str>,
        data_cols: Option<&[String]>,
    ) -> Result<DataFrame, ProcessorError> {
        match mode {
            DataMode::Single => {
                let data_type_col =
                    data_type_col.ok_or(ProcessorError::MissingSingleModeColumns)?;
                let value_col = value_col.ok_or(ProcessorError::MissingSingleModeColumns)?;

                // Build result manually
                let group_series = df.column(group_col)?;
                let dtype_series = df.column(data_type_col)?;
                let value_series = df.column(value_col)?;
                let value_f64 = value_series.cast(&DataType::Float64)?;
                let value_ca = value_f64.f64()?;

                let mut groups: Vec<String> = Vec::new();
                let mut data_types: Vec<String> = Vec::new();
                let mut values: Vec<f64> = Vec::new();

                for i in 0..df.height() {
                    if let (Ok(g), Ok(dt), Some(v)) =
                        (group_series.get(i), dtype_series.get(i), value_ca.get(i))
                    {
                        if !v.is_nan() && !g.is_null() && !dt.is_null() {
                            groups.push(g.to_string().trim_matches('"').to_string());
                            data_types.push(dt.to_string().trim_matches('"').to_string());
                            values.push(v);
                        }
                    }
                }

                let result = DataFrame::new(vec![
                    Column::new("group".into(), groups),
                    Column::new("data_type".into(), data_types),
                    Column::new("value".into(), values),
                ])?;

                Ok(result)
            }
            DataMode::Multi => {
                let data_cols = data_cols.ok_or(ProcessorError::MissingMultiModeColumns)?;
                if data_cols.is_empty() {
                    return Err(ProcessorError::MissingMultiModeColumns);
                }

                Self::stack_to_long(df, group_col, data_cols)
            }
        }
    }

    /// Get unique data types from processed DataFrame.
    pub fn get_data_types(df: &DataFrame) -> Vec<String> {
        df.column("data_type")
            .ok()
            .and_then(|col| col.unique().ok())
            .map(|unique| {
                let series = unique.as_materialized_series();
                let mut types: Vec<String> = (0..series.len())
                    .filter_map(|i| {
                        let val = series.get(i).ok()?;
                        if val.is_null() {
                            None
                        } else {
                            Some(val.to_string().trim_matches('"').to_string())
                        }
                    })
                    .collect();
                types.sort();
                types
            })
            .unwrap_or_default()
    }

    /// Get unique groups from processed DataFrame.
    pub fn get_groups(df: &DataFrame) -> Vec<String> {
        df.column("group")
            .ok()
            .and_then(|col| col.unique().ok())
            .map(|unique| {
                let series = unique.as_materialized_series();
                let mut groups: Vec<String> = (0..series.len())
                    .filter_map(|i| {
                        let val = series.get(i).ok()?;
                        if val.is_null() {
                            None
                        } else {
                            Some(val.to_string().trim_matches('"').to_string())
                        }
                    })
                    .collect();
                groups.sort();
                groups
            })
            .unwrap_or_default()
    }

    /// Filter DataFrame for a specific data type.
    pub fn filter_by_data_type(
        df: &DataFrame,
        data_type: &str,
    ) -> Result<DataFrame, ProcessorError> {
        let filtered = df
            .clone()
            .lazy()
            .filter(col("data_type").eq(lit(data_type)))
            .collect()?;
        Ok(filtered)
    }
}
