use calamine::DeError;
use thiserror::Error;

pub type ParserResult<T = (), E = ParserError> = Result<T, E>;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("unable to read file")]
    Io(#[from] std::io::Error),
    #[error("unable to parse file")]
    Parse(#[from] calamine::Error),
    #[error("Deserialize error: {0}")]
    Deserialize(#[from] DeError),
}
