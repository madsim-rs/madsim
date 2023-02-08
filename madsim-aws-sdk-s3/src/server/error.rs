use std::fmt::{Display, Formatter, Result as FmtResult};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidKey(String),
    InvalidBucket(String),
    InvalidUploadId(String),
    InvalidRangeSpecifier(String),
    InvalidPartNumberSpecifier(i32),
    GetBodyFailed,
    UnsupportRangeUnit(String),
    RequestTimeout,
}

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Error::InvalidKey(e) => write!(f, "invalid key: {e}"),
            Error::InvalidBucket(e) => write!(f, "invalid bucket: {e}"),
            Error::InvalidUploadId(e) => write!(f, "invalid upload_id: {e}"),
            Error::InvalidRangeSpecifier(e) => write!(f, "invalid range: {e}"),
            Error::InvalidPartNumberSpecifier(e) => write!(f, "invalid part_number: {e}"),
            Error::UnsupportRangeUnit(e) => write!(f, "unsupport range unit: {e}"),
            Error::RequestTimeout => write!(f, "request timed out"),
            Error::GetBodyFailed => write!(f, "get body failed"),
        }
    }
}

impl std::error::Error for Error {}
