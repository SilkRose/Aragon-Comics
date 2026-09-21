use std::error::Error as StdError;
use std::result::Result as StdResult;

pub type Error = Box<dyn StdError>;
pub type Result<T, E = Error> = StdResult<T, E>;
