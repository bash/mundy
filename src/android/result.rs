use std::sync::Arc;

pub(crate) type Result<T, E = BoxedError> = std::result::Result<T, E>;
pub(crate) type BoxedError = Box<dyn std::error::Error + Send + Sync>;
pub(crate) type ArcError = Arc<dyn std::error::Error + Send + Sync>;
