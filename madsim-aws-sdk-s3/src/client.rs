use aws_types::SdkConfig;

#[derive(std::fmt::Debug)]
pub struct Client {}

impl Clone for Client {
    fn clone(&self) -> Self {
        Self {}
    }
}

impl Client {
    pub fn create_multipart_upload(&self) -> fluent_builders::CreateMultipartUpload {
        fluent_builders::CreateMultipartUpload::new()
    }

    pub fn upload_part(&self) -> fluent_builders::UploadPart {
        fluent_builders::UploadPart::new()
    }

    pub fn complete_multipart_upload(&self) -> fluent_builders::CompleteMultipartUpload {
        fluent_builders::CompleteMultipartUpload::new()
    }

    pub fn abort_multipart_upload(&self) -> fluent_builders::AbortMultipartUpload {
        fluent_builders::AbortMultipartUpload::new()
    }

    pub fn get_object(&self) -> fluent_builders::GetObject {
        fluent_builders::GetObject::new()
    }

    pub fn put_object(&self) -> fluent_builders::PutObject {
        fluent_builders::PutObject::new()
    }

    pub fn delete_object(&self) -> fluent_builders::DeleteObject {
        fluent_builders::DeleteObject::new()
    }

    pub fn delete_objects(&self) -> fluent_builders::DeleteObjects {
        fluent_builders::DeleteObjects::new()
    }

    pub fn head_object(&self) -> fluent_builders::HeadObject {
        fluent_builders::HeadObject::new()
    }

    pub fn list_objects_v2(&self) -> fluent_builders::ListObjectsV2 {
        fluent_builders::ListObjectsV2::new()
    }
}

pub mod fluent_builders {
    use aws_smithy_client::SdkError;

    use crate::{
        error::{
            AbortMultipartUploadError, CreateMultipartUploadError, DeleteObjectError,
            DeleteObjectsError, GetObjectError, HeadObjectError, ListObjectsV2Error,
            PutObjectError, UploadPartError,
        },
        output::{
            AbortMultipartUploadOutput, CreateMultipartUploadOutput, DeleteObjectOutput,
            GetObjectOutput, HeadObjectOutput, ListObjectsV2Output, PutObjectOutput,
            UploadPartOutput,
        },
        types::ByteStream,
    };

    #[derive(std::fmt::Debug)]
    pub struct UploadPart {}
    impl UploadPart {
        pub(crate) fn new() -> Self {
            Self {}
        }

        pub async fn send(self) -> Result<UploadPartOutput, SdkError<UploadPartError>> {
            todo!();
        }

        pub fn body(mut self, _input: ByteStream) -> Self {
            todo!();
        }

        pub fn bucket(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn content_length(mut self, _input: i64) -> Self {
            todo!();
        }

        pub fn key(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn part_number(mut self, _input: i32) -> Self {
            todo!();
        }

        pub fn upload_id(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }
    }

    #[derive(Clone, Debug)]
    pub struct CompleteMultipartUpload {}
    impl CompleteMultipartUpload {
        pub(crate) fn new() -> Self {
            Self {}
        }

        pub async fn send(
            self,
        ) -> Result<CreateMultipartUploadOutput, SdkError<CreateMultipartUploadError>> {
            todo!();
        }

        pub fn bucket(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn key(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }
        pub fn multipart_upload(mut self, _input: crate::model::CompletedMultipartUpload) -> Self {
            todo!();
        }

        pub fn upload_id(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }
    }

    #[derive(Clone, Debug)]
    pub struct AbortMultipartUpload {}

    impl AbortMultipartUpload {
        pub(crate) fn new() -> Self {
            Self {}
        }

        pub async fn send(
            self,
        ) -> Result<AbortMultipartUploadOutput, SdkError<AbortMultipartUploadError>> {
            todo!();
        }

        pub fn bucket(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn key(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn upload_id(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }
    }

    #[derive(std::fmt::Debug)]
    pub struct GetObject {}
    impl GetObject {
        pub(crate) fn new() -> Self {
            Self {}
        }

        pub async fn send(self) -> Result<GetObjectOutput, SdkError<GetObjectError>> {
            todo!();
        }

        pub fn bucket(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn key(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn range(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }
    }
    #[derive(Debug)]
    pub struct PutObject {}
    impl PutObject {
        pub(crate) fn new() -> Self {
            Self {}
        }

        pub async fn send(self) -> Result<PutObjectOutput, SdkError<PutObjectError>> {
            todo!();
        }

        pub fn body(mut self, _input: ByteStream) -> Self {
            todo!();
        }

        pub fn bucket(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn key(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }
    }

    #[derive(Debug)]
    pub struct DeleteObject {}
    impl DeleteObject {
        pub(crate) fn new() -> Self {
            Self {}
        }

        pub async fn send(self) -> Result<DeleteObjectOutput, SdkError<DeleteObjectError>> {
            todo!();
        }

        pub fn bucket(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn key(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }
    }

    #[derive(Clone, Debug)]
    pub struct DeleteObjects {}
    impl DeleteObjects {
        pub(crate) fn new() -> Self {
            Self {}
        }

        pub async fn send(self) -> Result<DeleteObjectOutput, SdkError<DeleteObjectsError>> {
            todo!();
        }

        pub fn bucket(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn delete(mut self, _input: crate::model::Delete) -> Self {
            todo!();
        }
    }

    #[derive(std::clone::Clone, std::fmt::Debug)]
    pub struct CreateMultipartUpload {}
    impl CreateMultipartUpload {
        pub(crate) fn new() -> Self {
            Self {}
        }

        pub async fn send(
            self,
        ) -> Result<CreateMultipartUploadOutput, SdkError<CreateMultipartUploadError>> {
            todo!();
        }

        pub fn bucket(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn key(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }
    }

    #[derive(Clone, Debug)]
    pub struct HeadObject {}
    impl HeadObject {
        pub(crate) fn new() -> Self {
            Self {}
        }

        pub async fn send(self) -> Result<HeadObjectOutput, SdkError<HeadObjectError>> {
            todo!();
        }

        pub fn bucket(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn key(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }
    }

    #[derive(Clone, Debug)]
    pub struct ListObjectsV2 {}
    impl ListObjectsV2 {
        pub(crate) fn new() -> Self {
            Self {}
        }

        pub async fn send(self) -> Result<ListObjectsV2Output, SdkError<ListObjectsV2Error>> {
            todo!();
        }

        pub fn bucket(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn prefix(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }

        pub fn continuation_token(mut self, _input: impl Into<String>) -> Self {
            todo!();
        }
    }
}

impl Client {
    pub fn new(sdk_config: &SdkConfig) -> Self {
        Self::from_conf(sdk_config.into())
    }

    pub fn from_conf(_conf: crate::Config) -> Self {
        todo!();
    }
}
