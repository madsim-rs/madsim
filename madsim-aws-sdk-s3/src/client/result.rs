use crate::server::error::Error as ServerError;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Error as IOError;
use std::net::AddrParseError;
type BoxError = Box<dyn Error + Send + Sync>;

#[derive(Debug)]
pub enum SdkError<E> {
    ConstructionFailure(BoxError),

    TimeoutError(BoxError),

    DispatchFailure(ConnectorError),

    ResponseError(BoxError),

    ServiceError(E),
}

use SdkError::*;

#[derive(Debug)]
pub struct ConnectorError {
    err: BoxError,
    kind: ConnectorErrorKind,
}

impl Display for ConnectorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}: {}", self.kind, self.err)
    }
}

impl Error for ConnectorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.err.as_ref())
    }
}

impl ConnectorError {
    pub fn timeout(err: BoxError) -> Self {
        Self {
            err,
            kind: ConnectorErrorKind::Timeout,
        }
    }

    pub fn user(err: BoxError) -> Self {
        Self {
            err,
            kind: ConnectorErrorKind::User,
        }
    }

    pub fn io(err: BoxError) -> Self {
        Self {
            err,
            kind: ConnectorErrorKind::Io,
        }
    }

    pub fn other(err: BoxError, kind: Option<ErrorKind>) -> Self {
        Self {
            err,
            kind: ConnectorErrorKind::Other(kind),
        }
    }

    pub fn is_io(&self) -> bool {
        matches!(self.kind, ConnectorErrorKind::Io)
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self.kind, ConnectorErrorKind::Timeout)
    }

    pub fn is_user(&self) -> bool {
        matches!(self.kind, ConnectorErrorKind::User)
    }

    pub fn is_other(&self) -> Option<ErrorKind> {
        match &self.kind {
            ConnectorErrorKind::Other(ek) => *ek,
            _ => None,
        }
    }
}

#[derive(Debug)]
enum ConnectorErrorKind {
    Timeout,

    User,

    Io,

    Other(Option<ErrorKind>),
}

impl Display for ConnectorErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ConnectorErrorKind::Timeout => write!(f, "timeout"),
            ConnectorErrorKind::User => write!(f, "user error"),
            ConnectorErrorKind::Io => write!(f, "io error"),
            ConnectorErrorKind::Other(Some(kind)) => write!(f, "{:?}", kind),
            ConnectorErrorKind::Other(None) => write!(f, "other"),
        }
    }
}

impl<E> Display for SdkError<E>
where
    E: Error,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            SdkError::ConstructionFailure(err) => write!(f, "failed to construct request: {}", err),
            SdkError::TimeoutError(err) => write!(f, "request has timed out: {}", err),
            SdkError::DispatchFailure(err) => Display::fmt(&err, f),
            SdkError::ResponseError(err) => Display::fmt(&err, f),
            SdkError::ServiceError(err) => Display::fmt(&err, f),
        }
    }
}

impl<E> Error for SdkError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use SdkError::*;
        match self {
            ConstructionFailure(err) | TimeoutError(err) | ResponseError(err) => Some(err.as_ref()),
            DispatchFailure(err) => Some(err),
            ServiceError(err) => Some(err),
        }
    }
}

impl<E> From<IOError> for SdkError<E> {
    #[inline]
    fn from(e: IOError) -> Self {
        ConstructionFailure(Box::new(e))
    }
}

impl<E> From<AddrParseError> for SdkError<E> {
    #[inline]
    fn from(e: AddrParseError) -> Self {
        ConstructionFailure(Box::new(e))
    }
}

impl<E> From<ServerError> for SdkError<E> {
    #[inline]
    fn from(e: ServerError) -> Self {
        ConstructionFailure(Box::new(e))
    }
}

impl<E> From<aws_smithy_http::operation::BuildError> for SdkError<E> {
    #[inline]
    fn from(e: aws_smithy_http::operation::BuildError) -> Self {
        ConstructionFailure(Box::new(e))
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    TransientError,

    ThrottlingError,

    ServerError,

    ClientError,
}
