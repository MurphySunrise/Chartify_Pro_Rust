//! CSV Data Loader Module
//! Handles CSV file loading and column extraction using Polars.

use polars::prelude::*;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoaderError {
    #[error("Failed to load CSV: {0}")]
    CsvError(#[from] PolarsError),
    #[error("No data loaded")]
    NoData,
}

/// Handles CSV file loading with Polars for high performance.
pub struct DataLoader {
    df: Option<DataFrame>,
    file_path: Option<PathBuf>,
}

impl Default for DataLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl DataLoader {
    pub fn new() -> Self {
        Self {
            df: None,
            file_path: None,
        }
    }

    /// Load a CSV file using Polars.
    pub fn load_csv(&mut self, file_path: &str) -> Result<&DataFrame, LoaderError> {
        self.file_path = Some(PathBuf::from(file_path));

        // Use lazy evaluation for memory efficiency, then collect
        let df = LazyCsvReader::new(file_path)
            .with_infer_schema_length(Some(10000))
            .with_ignore_errors(true)
            .finish()?
            .collect()?;

        self.df = Some(df);
        self.df.as_ref().ok_or(LoaderError::NoData)
    }

    /// Get list of column names from loaded DataFrame.
    pub fn get_columns(&self) -> Vec<String> {
        self.df
            .as_ref()
            .map(|df| {
                df.get_column_names()
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get list of numeric column names.
    pub fn get_numeric_columns(&self) -> Vec<String> {
        let Some(df) = &self.df else {
            return Vec::new();
        };

        df.get_columns()
            .iter()
            .filter(|col| {
                matches!(
                    col.dtype(),
                    DataType::Float32
                        | DataType::Float64
                        | DataType::Int8
                        | DataType::Int16
                        | DataType::Int32
                        | DataType::Int64
                        | DataType::UInt8
                        | DataType::UInt16
                        | DataType::UInt32
                        | DataType::UInt64
                )
            })
            .map(|col| col.name().to_string())
            .collect()
    }

    /// Get unique values from a column.
    pub fn get_unique_values(&self, column: &str) -> Vec<String> {
        let Some(df) = &self.df else {
            return Vec::new();
        };

        df.column(column)
            .ok()
            .and_then(|col| col.unique().ok())
            .map(|unique| {
                let series = unique.as_materialized_series();
                (0..series.len())
                    .filter_map(|i| {
                        let val = series.get(i).ok()?;
                        if val.is_null() {
                            None
                        } else {
                            Some(val.to_string().trim_matches('"').to_string())
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the number of rows in the DataFrame.
    pub fn get_row_count(&self) -> usize {
        self.df.as_ref().map(|df| df.height()).unwrap_or(0)
    }

    /// Get a reference to the loaded DataFrame.
    pub fn get_dataframe(&self) -> Option<&DataFrame> {
        self.df.as_ref()
    }

    /// Get file path.
    pub fn get_file_path(&self) -> Option<&PathBuf> {
        self.file_path.as_ref()
    }

    /// Set DataFrame directly (used for async loading)
    pub fn set_dataframe(&mut self, df: DataFrame) {
        self.df = Some(df);
    }
}
