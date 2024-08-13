pub type Result<T=(), E=Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
  #[error("Value exceeds max safe decimal")]
  Overflow {},

  #[error("Unexpected decimal format")]
  UnexpectedFormat {},

  #[error(transparent)]
  ParseInt(#[from] std::num::ParseIntError),
}
