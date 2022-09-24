use std::fmt::{Display, Formatter};

#[non_exhaustive]
#[derive(Debug)]
pub struct UploadPartError {
    pub kind: UploadPartErrorKind,
    pub(crate) meta: aws_smithy_types::Error,
}
#[non_exhaustive]
#[derive(Debug)]
pub enum UploadPartErrorKind {
    Unhandled(Box<dyn std::error::Error + Send + Sync + 'static>),
}
impl Display for UploadPartError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            UploadPartErrorKind::Unhandled(_inner) => _inner.fmt(f),
        }
    }
}
impl aws_smithy_types::retry::ProvideErrorKind for UploadPartError {
    fn code(&self) -> Option<&str> {
        UploadPartError::code(self)
    }
    fn retryable_error_kind(&self) -> Option<aws_smithy_types::retry::ErrorKind> {
        None
    }
}
impl UploadPartError {
    pub fn new(kind: UploadPartErrorKind, meta: aws_smithy_types::Error) -> Self {
        Self { kind, meta }
    }

    pub fn unhandled(err: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self {
            kind: UploadPartErrorKind::Unhandled(err.into()),
            meta: Default::default(),
        }
    }

    pub fn generic(err: aws_smithy_types::Error) -> Self {
        Self {
            meta: err.clone(),
            kind: UploadPartErrorKind::Unhandled(err.into()),
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.meta.message()
    }

    pub fn meta(&self) -> &aws_smithy_types::Error {
        &self.meta
    }

    pub fn request_id(&self) -> Option<&str> {
        self.meta.request_id()
    }

    pub fn code(&self) -> Option<&str> {
        self.meta.code()
    }
}
impl std::error::Error for UploadPartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            UploadPartErrorKind::Unhandled(_inner) => Some(_inner.as_ref()),
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct CompleteMultipartUploadError {
    pub kind: CompleteMultipartUploadErrorKind,
    pub(crate) meta: aws_smithy_types::Error,
}
#[non_exhaustive]
#[derive(Debug)]
pub enum CompleteMultipartUploadErrorKind {
    Unhandled(Box<dyn std::error::Error + Send + Sync + 'static>),
}
impl Display for CompleteMultipartUploadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            CompleteMultipartUploadErrorKind::Unhandled(_inner) => _inner.fmt(f),
        }
    }
}
impl aws_smithy_types::retry::ProvideErrorKind for CompleteMultipartUploadError {
    fn code(&self) -> Option<&str> {
        CompleteMultipartUploadError::code(self)
    }
    fn retryable_error_kind(&self) -> Option<aws_smithy_types::retry::ErrorKind> {
        None
    }
}
impl CompleteMultipartUploadError {
    pub fn new(kind: CompleteMultipartUploadErrorKind, meta: aws_smithy_types::Error) -> Self {
        Self { kind, meta }
    }

    pub fn unhandled(err: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self {
            kind: CompleteMultipartUploadErrorKind::Unhandled(err.into()),
            meta: Default::default(),
        }
    }

    pub fn generic(err: aws_smithy_types::Error) -> Self {
        Self {
            meta: err.clone(),
            kind: CompleteMultipartUploadErrorKind::Unhandled(err.into()),
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.meta.message()
    }

    pub fn meta(&self) -> &aws_smithy_types::Error {
        &self.meta
    }

    pub fn request_id(&self) -> Option<&str> {
        self.meta.request_id()
    }

    pub fn code(&self) -> Option<&str> {
        self.meta.code()
    }
}
impl std::error::Error for CompleteMultipartUploadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            CompleteMultipartUploadErrorKind::Unhandled(_inner) => Some(_inner.as_ref()),
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct AbortMultipartUploadError {
    pub kind: AbortMultipartUploadErrorKind,
    pub(crate) meta: aws_smithy_types::Error,
}
/// Types of errors that can occur for the `AbortMultipartUpload` operation.
#[non_exhaustive]
#[derive(Debug)]
pub enum AbortMultipartUploadErrorKind {
    NoSuchUpload(crate::error::NoSuchUpload),
    Unhandled(Box<dyn std::error::Error + Send + Sync + 'static>),
}
impl std::fmt::Display for AbortMultipartUploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            AbortMultipartUploadErrorKind::NoSuchUpload(_inner) => _inner.fmt(f),
            AbortMultipartUploadErrorKind::Unhandled(_inner) => _inner.fmt(f),
        }
    }
}
impl aws_smithy_types::retry::ProvideErrorKind for AbortMultipartUploadError {
    fn code(&self) -> Option<&str> {
        AbortMultipartUploadError::code(self)
    }
    fn retryable_error_kind(&self) -> Option<aws_smithy_types::retry::ErrorKind> {
        None
    }
}
impl AbortMultipartUploadError {
    pub fn new(kind: AbortMultipartUploadErrorKind, meta: aws_smithy_types::Error) -> Self {
        Self { kind, meta }
    }

    pub fn unhandled(err: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self {
            kind: AbortMultipartUploadErrorKind::Unhandled(err.into()),
            meta: Default::default(),
        }
    }

    pub fn generic(err: aws_smithy_types::Error) -> Self {
        Self {
            meta: err.clone(),
            kind: AbortMultipartUploadErrorKind::Unhandled(err.into()),
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.meta.message()
    }

    pub fn meta(&self) -> &aws_smithy_types::Error {
        &self.meta
    }

    pub fn request_id(&self) -> Option<&str> {
        self.meta.request_id()
    }

    pub fn code(&self) -> Option<&str> {
        self.meta.code()
    }
    pub fn is_no_such_upload(&self) -> bool {
        matches!(&self.kind, AbortMultipartUploadErrorKind::NoSuchUpload(_))
    }
}
impl std::error::Error for AbortMultipartUploadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            AbortMultipartUploadErrorKind::NoSuchUpload(_inner) => Some(_inner),
            AbortMultipartUploadErrorKind::Unhandled(_inner) => Some(_inner.as_ref()),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct NoSuchUpload {
    pub message: Option<String>,
}
impl std::fmt::Debug for NoSuchUpload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("NoSuchUpload");
        formatter.field("message", &self.message);
        formatter.finish()
    }
}
impl NoSuchUpload {
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}
impl std::fmt::Display for NoSuchUpload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoSuchUpload")?;
        if let Some(inner_9) = &self.message {
            write!(f, ": {}", inner_9)?;
        }
        Ok(())
    }
}
impl std::error::Error for NoSuchUpload {}
pub mod no_such_upload {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) message: Option<String>,
    }
    impl Builder {
        pub fn message(mut self, input: impl Into<String>) -> Self {
            self.message = Some(input.into());
            self
        }
        pub fn set_message(mut self, input: Option<String>) -> Self {
            self.message = input;
            self
        }
        pub fn build(self) -> crate::error::NoSuchUpload {
            crate::error::NoSuchUpload {
                message: self.message,
            }
        }
    }
}
impl NoSuchUpload {
    pub fn builder() -> crate::error::no_such_upload::Builder {
        crate::error::no_such_upload::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct GetObjectError {
    pub kind: GetObjectErrorKind,
    pub(crate) meta: aws_smithy_types::Error,
}
#[non_exhaustive]
#[derive(Debug)]
pub enum GetObjectErrorKind {
    InvalidObjectState(crate::error::InvalidObjectState),
    NoSuchKey(crate::error::NoSuchKey),
    Unhandled(Box<dyn std::error::Error + Send + Sync + 'static>),
}
impl std::fmt::Display for GetObjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            GetObjectErrorKind::InvalidObjectState(_inner) => _inner.fmt(f),
            GetObjectErrorKind::NoSuchKey(_inner) => _inner.fmt(f),
            GetObjectErrorKind::Unhandled(_inner) => _inner.fmt(f),
        }
    }
}
impl aws_smithy_types::retry::ProvideErrorKind for GetObjectError {
    fn code(&self) -> Option<&str> {
        GetObjectError::code(self)
    }
    fn retryable_error_kind(&self) -> Option<aws_smithy_types::retry::ErrorKind> {
        None
    }
}
impl GetObjectError {
    pub fn new(kind: GetObjectErrorKind, meta: aws_smithy_types::Error) -> Self {
        Self { kind, meta }
    }

    pub fn unhandled(err: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self {
            kind: GetObjectErrorKind::Unhandled(err.into()),
            meta: Default::default(),
        }
    }

    pub fn generic(err: aws_smithy_types::Error) -> Self {
        Self {
            meta: err.clone(),
            kind: GetObjectErrorKind::Unhandled(err.into()),
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.meta.message()
    }

    pub fn meta(&self) -> &aws_smithy_types::Error {
        &self.meta
    }

    pub fn request_id(&self) -> Option<&str> {
        self.meta.request_id()
    }

    pub fn code(&self) -> Option<&str> {
        self.meta.code()
    }
    pub fn is_invalid_object_state(&self) -> bool {
        matches!(&self.kind, GetObjectErrorKind::InvalidObjectState(_))
    }
    pub fn is_no_such_key(&self) -> bool {
        matches!(&self.kind, GetObjectErrorKind::NoSuchKey(_))
    }
}
impl std::error::Error for GetObjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            GetObjectErrorKind::InvalidObjectState(_inner) => Some(_inner),
            GetObjectErrorKind::NoSuchKey(_inner) => Some(_inner),
            GetObjectErrorKind::Unhandled(_inner) => Some(_inner.as_ref()),
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct PutObjectError {
    pub kind: PutObjectErrorKind,
    pub(crate) meta: aws_smithy_types::Error,
}
#[non_exhaustive]
#[derive(Debug)]
pub enum PutObjectErrorKind {
    Unhandled(Box<dyn std::error::Error + Send + Sync + 'static>),
}
impl std::fmt::Display for PutObjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            PutObjectErrorKind::Unhandled(_inner) => _inner.fmt(f),
        }
    }
}
impl aws_smithy_types::retry::ProvideErrorKind for PutObjectError {
    fn code(&self) -> Option<&str> {
        PutObjectError::code(self)
    }
    fn retryable_error_kind(&self) -> Option<aws_smithy_types::retry::ErrorKind> {
        None
    }
}
impl PutObjectError {
    pub fn new(kind: PutObjectErrorKind, meta: aws_smithy_types::Error) -> Self {
        Self { kind, meta }
    }

    pub fn unhandled(err: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self {
            kind: PutObjectErrorKind::Unhandled(err.into()),
            meta: Default::default(),
        }
    }

    pub fn generic(err: aws_smithy_types::Error) -> Self {
        Self {
            meta: err.clone(),
            kind: PutObjectErrorKind::Unhandled(err.into()),
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.meta.message()
    }

    pub fn meta(&self) -> &aws_smithy_types::Error {
        &self.meta
    }

    pub fn request_id(&self) -> Option<&str> {
        self.meta.request_id()
    }

    pub fn code(&self) -> Option<&str> {
        self.meta.code()
    }
}
impl std::error::Error for PutObjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            PutObjectErrorKind::Unhandled(_inner) => Some(_inner.as_ref()),
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct DeleteObjectError {
    pub kind: DeleteObjectErrorKind,
    pub(crate) meta: aws_smithy_types::Error,
}
#[non_exhaustive]
#[derive(Debug)]
pub enum DeleteObjectErrorKind {
    Unhandled(Box<dyn std::error::Error + Send + Sync + 'static>),
}
impl std::fmt::Display for DeleteObjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            DeleteObjectErrorKind::Unhandled(_inner) => _inner.fmt(f),
        }
    }
}
impl aws_smithy_types::retry::ProvideErrorKind for DeleteObjectError {
    fn code(&self) -> Option<&str> {
        DeleteObjectError::code(self)
    }
    fn retryable_error_kind(&self) -> Option<aws_smithy_types::retry::ErrorKind> {
        None
    }
}
impl DeleteObjectError {
    pub fn new(kind: DeleteObjectErrorKind, meta: aws_smithy_types::Error) -> Self {
        Self { kind, meta }
    }

    pub fn unhandled(err: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self {
            kind: DeleteObjectErrorKind::Unhandled(err.into()),
            meta: Default::default(),
        }
    }

    pub fn generic(err: aws_smithy_types::Error) -> Self {
        Self {
            meta: err.clone(),
            kind: DeleteObjectErrorKind::Unhandled(err.into()),
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.meta.message()
    }

    pub fn meta(&self) -> &aws_smithy_types::Error {
        &self.meta
    }

    pub fn request_id(&self) -> Option<&str> {
        self.meta.request_id()
    }

    pub fn code(&self) -> Option<&str> {
        self.meta.code()
    }
}
impl std::error::Error for DeleteObjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            DeleteObjectErrorKind::Unhandled(_inner) => Some(_inner.as_ref()),
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct DeleteObjectsError {
    pub kind: DeleteObjectsErrorKind,
    pub(crate) meta: aws_smithy_types::Error,
}
#[non_exhaustive]
#[derive(Debug)]
pub enum DeleteObjectsErrorKind {
    Unhandled(Box<dyn std::error::Error + Send + Sync + 'static>),
}
impl std::fmt::Display for DeleteObjectsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            DeleteObjectsErrorKind::Unhandled(_inner) => _inner.fmt(f),
        }
    }
}
impl aws_smithy_types::retry::ProvideErrorKind for DeleteObjectsError {
    fn code(&self) -> Option<&str> {
        DeleteObjectsError::code(self)
    }
    fn retryable_error_kind(&self) -> Option<aws_smithy_types::retry::ErrorKind> {
        None
    }
}
impl DeleteObjectsError {
    pub fn new(kind: DeleteObjectsErrorKind, meta: aws_smithy_types::Error) -> Self {
        Self { kind, meta }
    }

    pub fn unhandled(err: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self {
            kind: DeleteObjectsErrorKind::Unhandled(err.into()),
            meta: Default::default(),
        }
    }

    pub fn generic(err: aws_smithy_types::Error) -> Self {
        Self {
            meta: err.clone(),
            kind: DeleteObjectsErrorKind::Unhandled(err.into()),
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.meta.message()
    }

    pub fn meta(&self) -> &aws_smithy_types::Error {
        &self.meta
    }

    pub fn request_id(&self) -> Option<&str> {
        self.meta.request_id()
    }

    pub fn code(&self) -> Option<&str> {
        self.meta.code()
    }
}
impl std::error::Error for DeleteObjectsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            DeleteObjectsErrorKind::Unhandled(_inner) => Some(_inner.as_ref()),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct InvalidObjectState {}
impl InvalidObjectState {}
impl std::fmt::Debug for InvalidObjectState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("InvalidObjectState");
        formatter.finish()
    }
}

impl std::fmt::Display for InvalidObjectState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InvalidObjectState")?;
        Ok(())
    }
}
impl std::error::Error for InvalidObjectState {}
pub mod invalid_object_state {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {}
    impl Builder {
        pub fn build(self) -> crate::error::InvalidObjectState {
            crate::error::InvalidObjectState {}
        }
    }
}
impl InvalidObjectState {
    pub fn builder() -> crate::error::invalid_object_state::Builder {
        crate::error::invalid_object_state::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct NoSuchKey {
    pub message: Option<String>,
}
impl std::fmt::Debug for NoSuchKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("NoSuchKey");
        formatter.field("message", &self.message);
        formatter.finish()
    }
}
impl NoSuchKey {
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}
impl std::fmt::Display for NoSuchKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoSuchKey")?;
        if let Some(inner_2) = &self.message {
            write!(f, ": {}", inner_2)?;
        }
        Ok(())
    }
}
impl std::error::Error for NoSuchKey {}
pub mod no_such_key {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) message: Option<String>,
    }
    impl Builder {
        pub fn message(mut self, input: impl Into<String>) -> Self {
            self.message = Some(input.into());
            self
        }
        pub fn set_message(mut self, input: Option<String>) -> Self {
            self.message = input;
            self
        }
        pub fn build(self) -> crate::error::NoSuchKey {
            crate::error::NoSuchKey {
                message: self.message,
            }
        }
    }
}
impl NoSuchKey {
    pub fn builder() -> crate::error::no_such_key::Builder {
        crate::error::no_such_key::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct CreateMultipartUploadError {
    pub kind: CreateMultipartUploadErrorKind,
    pub(crate) meta: aws_smithy_types::Error,
}
#[non_exhaustive]
#[derive(Debug)]
pub enum CreateMultipartUploadErrorKind {
    Unhandled(Box<dyn std::error::Error + Send + Sync + 'static>),
}
impl std::fmt::Display for CreateMultipartUploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            CreateMultipartUploadErrorKind::Unhandled(_inner) => _inner.fmt(f),
        }
    }
}
impl aws_smithy_types::retry::ProvideErrorKind for CreateMultipartUploadError {
    fn code(&self) -> Option<&str> {
        CreateMultipartUploadError::code(self)
    }
    fn retryable_error_kind(&self) -> Option<aws_smithy_types::retry::ErrorKind> {
        None
    }
}
impl CreateMultipartUploadError {
    pub fn new(kind: CreateMultipartUploadErrorKind, meta: aws_smithy_types::Error) -> Self {
        Self { kind, meta }
    }

    pub fn unhandled(err: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self {
            kind: CreateMultipartUploadErrorKind::Unhandled(err.into()),
            meta: Default::default(),
        }
    }

    pub fn generic(err: aws_smithy_types::Error) -> Self {
        Self {
            meta: err.clone(),
            kind: CreateMultipartUploadErrorKind::Unhandled(err.into()),
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.meta.message()
    }

    pub fn meta(&self) -> &aws_smithy_types::Error {
        &self.meta
    }

    pub fn request_id(&self) -> Option<&str> {
        self.meta.request_id()
    }

    pub fn code(&self) -> Option<&str> {
        self.meta.code()
    }
}
impl std::error::Error for CreateMultipartUploadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            CreateMultipartUploadErrorKind::Unhandled(_inner) => Some(_inner.as_ref()),
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct HeadObjectError {
    pub kind: HeadObjectErrorKind,
    pub(crate) meta: aws_smithy_types::Error,
}
/// Types of errors that can occur for the `HeadObject` operation.
#[non_exhaustive]
#[derive(Debug)]
pub enum HeadObjectErrorKind {
    /// <p>The specified content does not exist.</p>
    NotFound(crate::error::NotFound),
    Unhandled(Box<dyn std::error::Error + Send + Sync + 'static>),
}
impl std::fmt::Display for HeadObjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            HeadObjectErrorKind::NotFound(_inner) => _inner.fmt(f),
            HeadObjectErrorKind::Unhandled(_inner) => _inner.fmt(f),
        }
    }
}
impl aws_smithy_types::retry::ProvideErrorKind for HeadObjectError {
    fn code(&self) -> Option<&str> {
        HeadObjectError::code(self)
    }
    fn retryable_error_kind(&self) -> Option<aws_smithy_types::retry::ErrorKind> {
        None
    }
}
impl HeadObjectError {
    pub fn new(kind: HeadObjectErrorKind, meta: aws_smithy_types::Error) -> Self {
        Self { kind, meta }
    }

    pub fn unhandled(err: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self {
            kind: HeadObjectErrorKind::Unhandled(err.into()),
            meta: Default::default(),
        }
    }

    pub fn generic(err: aws_smithy_types::Error) -> Self {
        Self {
            meta: err.clone(),
            kind: HeadObjectErrorKind::Unhandled(err.into()),
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.meta.message()
    }

    pub fn meta(&self) -> &aws_smithy_types::Error {
        &self.meta
    }

    pub fn request_id(&self) -> Option<&str> {
        self.meta.request_id()
    }

    pub fn code(&self) -> Option<&str> {
        self.meta.code()
    }
    pub fn is_not_found(&self) -> bool {
        matches!(&self.kind, HeadObjectErrorKind::NotFound(_))
    }
}
impl std::error::Error for HeadObjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            HeadObjectErrorKind::NotFound(_inner) => Some(_inner),
            HeadObjectErrorKind::Unhandled(_inner) => Some(_inner.as_ref()),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct NotFound {
    pub message: Option<String>,
}
impl std::fmt::Debug for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("NotFound");
        formatter.field("message", &self.message);
        formatter.finish()
    }
}
impl NotFound {
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}
impl std::fmt::Display for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NotFound")?;
        if let Some(inner_4) = &self.message {
            write!(f, ": {}", inner_4)?;
        }
        Ok(())
    }
}
impl std::error::Error for NotFound {}
pub mod not_found {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) message: Option<String>,
    }
    impl Builder {
        pub fn message(mut self, input: impl Into<String>) -> Self {
            self.message = Some(input.into());
            self
        }
        pub fn set_message(mut self, input: Option<String>) -> Self {
            self.message = input;
            self
        }
        pub fn build(self) -> crate::error::NotFound {
            crate::error::NotFound {
                message: self.message,
            }
        }
    }
}
impl NotFound {
    pub fn builder() -> crate::error::not_found::Builder {
        crate::error::not_found::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct ListObjectsV2Error {
    pub kind: ListObjectsV2ErrorKind,
    pub(crate) meta: aws_smithy_types::Error,
}
#[non_exhaustive]
#[derive(Debug)]
pub enum ListObjectsV2ErrorKind {
    NoSuchBucket(crate::error::NoSuchBucket),
    Unhandled(Box<dyn std::error::Error + Send + Sync + 'static>),
}
impl std::fmt::Display for ListObjectsV2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ListObjectsV2ErrorKind::NoSuchBucket(_inner) => _inner.fmt(f),
            ListObjectsV2ErrorKind::Unhandled(_inner) => _inner.fmt(f),
        }
    }
}
impl aws_smithy_types::retry::ProvideErrorKind for ListObjectsV2Error {
    fn code(&self) -> Option<&str> {
        ListObjectsV2Error::code(self)
    }
    fn retryable_error_kind(&self) -> Option<aws_smithy_types::retry::ErrorKind> {
        None
    }
}
impl ListObjectsV2Error {
    pub fn new(kind: ListObjectsV2ErrorKind, meta: aws_smithy_types::Error) -> Self {
        Self { kind, meta }
    }

    pub fn unhandled(err: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self {
            kind: ListObjectsV2ErrorKind::Unhandled(err.into()),
            meta: Default::default(),
        }
    }

    pub fn generic(err: aws_smithy_types::Error) -> Self {
        Self {
            meta: err.clone(),
            kind: ListObjectsV2ErrorKind::Unhandled(err.into()),
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.meta.message()
    }

    pub fn meta(&self) -> &aws_smithy_types::Error {
        &self.meta
    }

    pub fn request_id(&self) -> Option<&str> {
        self.meta.request_id()
    }

    pub fn code(&self) -> Option<&str> {
        self.meta.code()
    }
    pub fn is_no_such_bucket(&self) -> bool {
        matches!(&self.kind, ListObjectsV2ErrorKind::NoSuchBucket(_))
    }
}
impl std::error::Error for ListObjectsV2Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ListObjectsV2ErrorKind::NoSuchBucket(_inner) => Some(_inner),
            ListObjectsV2ErrorKind::Unhandled(_inner) => Some(_inner.as_ref()),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct NoSuchBucket {
    pub message: Option<String>,
}
impl std::fmt::Debug for NoSuchBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("NoSuchBucket");
        formatter.field("message", &self.message);
        formatter.finish()
    }
}
impl NoSuchBucket {
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}
impl std::fmt::Display for NoSuchBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoSuchBucket")?;
        if let Some(inner_3) = &self.message {
            write!(f, ": {}", inner_3)?;
        }
        Ok(())
    }
}
impl std::error::Error for NoSuchBucket {}
pub mod no_such_bucket {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) message: Option<String>,
    }
    impl Builder {
        pub fn message(mut self, input: impl Into<String>) -> Self {
            self.message = Some(input.into());
            self
        }
        pub fn set_message(mut self, input: Option<String>) -> Self {
            self.message = input;
            self
        }
        pub fn build(self) -> crate::error::NoSuchBucket {
            crate::error::NoSuchBucket {
                message: self.message,
            }
        }
    }
}
impl NoSuchBucket {
    pub fn builder() -> crate::error::no_such_bucket::Builder {
        crate::error::no_such_bucket::Builder::default()
    }
}
