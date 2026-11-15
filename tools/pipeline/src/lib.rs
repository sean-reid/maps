pub mod build;
pub mod catalogue;
pub mod derive;
pub mod fao;
pub mod fetch;
pub mod format;
pub mod index;
pub mod rates;
pub mod wdi;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
