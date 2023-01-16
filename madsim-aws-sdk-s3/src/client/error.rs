use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug)]
pub struct UploadPartError {
    pub kind: String,
}

impl Display for UploadPartError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for UploadPartError {}

#[derive(Debug)]
pub struct CompleteMultipartUploadError {
    pub kind: String,
}

impl Display for CompleteMultipartUploadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for CompleteMultipartUploadError {}

#[derive(Debug)]
pub struct AbortMultipartUploadError {
    pub kind: String,
}

impl Display for AbortMultipartUploadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for AbortMultipartUploadError {}

#[derive(Debug)]
pub struct GetObjectError {
    pub kind: String,
}

impl Display for GetObjectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for GetObjectError {}

#[derive(Debug)]
pub struct PutObjectError {
    pub kind: String,
}

impl Display for PutObjectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for PutObjectError {}

#[derive(Debug)]
pub struct DeleteObjectError {
    pub kind: String,
}

impl Display for DeleteObjectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for DeleteObjectError {}

#[derive(Debug)]
pub struct DeleteObjectsError {
    pub kind: String,
}

impl Display for DeleteObjectsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for DeleteObjectsError {}

#[derive(Debug)]
pub struct CreateMultipartUploadError {
    pub kind: String,
}

impl Display for CreateMultipartUploadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for CreateMultipartUploadError {}

#[derive(Debug)]
pub struct HeadObjectError {
    pub kind: String,
}

impl Display for HeadObjectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for HeadObjectError {}

#[derive(Debug)]
pub struct ListObjectsV2Error {
    pub kind: String,
}

impl Display for ListObjectsV2Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for ListObjectsV2Error {}

#[derive(Debug)]
pub struct PutBucketLifecycleConfigurationError {
    pub kind: String,
}

impl Display for PutBucketLifecycleConfigurationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for PutBucketLifecycleConfigurationError {}

#[derive(Debug)]
pub struct GetBucketLifecycleConfigurationError {
    pub kind: String,
}

impl Display for GetBucketLifecycleConfigurationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for GetBucketLifecycleConfigurationError {}
