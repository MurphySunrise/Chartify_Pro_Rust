//! Data module - CSV loading and processing

mod loader;
mod processor;

pub use loader::DataLoader;
pub use processor::{DataProcessor, DataMode};
