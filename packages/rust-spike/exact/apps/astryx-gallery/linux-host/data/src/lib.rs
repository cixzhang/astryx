//! The gallery's data source: the gallery reads no data, so every query is refused.

use exact_runner::{DataError, DataSource, Value};

/// The gallery's data source.
#[derive(Default)]
pub struct Gallery;

impl DataSource for Gallery {
    fn app_id(&self) -> &str {
        "com.example.astryx-gallery"
    }

    fn query(&mut self, source: &str, _args: &[Value]) -> Result<Value, DataError> {
        Err(DataError::UnknownSource(source.to_string()))
    }
}
