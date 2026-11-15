pub mod fetch;
pub mod format;
pub mod index;
pub mod wdi;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
