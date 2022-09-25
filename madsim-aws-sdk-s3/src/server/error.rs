use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidKey(String),
    InvalidBucket(String),
    InvalidUploadId(String),
}

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidKey(e) => write!(f, "invalid key: {}", e),
            Error::InvalidBucket(e) => write!(f, "invalid bucket: {}", e),
            Error::InvalidUploadId(e) => write!(f, "invalid upload_id: {}", e),
        }
    }
}

impl std::error::Error for Error {}
