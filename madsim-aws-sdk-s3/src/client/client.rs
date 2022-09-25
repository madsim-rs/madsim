use std::sync::Arc;

use aws_types::SdkConfig;
use madsim::net::Endpoint;

pub(crate) struct Handle {
    pub(crate) endpoint: Endpoint,
    pub(crate) conf: crate::Config,
}

pub struct Client {
    handle: Arc<Handle>,
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

impl From<Endpoint> for Client {
    fn from(ep: Endpoint) -> Self {
        Self::with_config(ep, crate::Config::builder().build())
    }
}

impl Client {
    pub fn with_config(endpoint: Endpoint, conf: crate::Config) -> Self {
        Self {
            handle: Arc::new(Handle { endpoint, conf }),
        }
    }

    pub fn conf(&self) -> &crate::Config {
        &self.handle.conf
    }
}

impl Client {
    pub fn create_multipart_upload(&self) -> fluent_builders::CreateMultipartUpload {
        fluent_builders::CreateMultipartUpload::new(self.handle.clone())
    }

    pub fn upload_part(&self) -> fluent_builders::UploadPart {
        fluent_builders::UploadPart::new(self.handle.clone())
    }

    pub fn complete_multipart_upload(&self) -> fluent_builders::CompleteMultipartUpload {
        fluent_builders::CompleteMultipartUpload::new(self.handle.clone())
    }

    pub fn abort_multipart_upload(&self) -> fluent_builders::AbortMultipartUpload {
        fluent_builders::AbortMultipartUpload::new(self.handle.clone())
    }

    pub fn get_object(&self) -> fluent_builders::GetObject {
        fluent_builders::GetObject::new(self.handle.clone())
    }

    pub fn put_object(&self) -> fluent_builders::PutObject {
        fluent_builders::PutObject::new(self.handle.clone())
    }

    pub fn delete_object(&self) -> fluent_builders::DeleteObject {
        fluent_builders::DeleteObject::new(self.handle.clone())
    }

    pub fn delete_objects(&self) -> fluent_builders::DeleteObjects {
        fluent_builders::DeleteObjects::new(self.handle.clone())
    }

    pub fn head_object(&self) -> fluent_builders::HeadObject {
        fluent_builders::HeadObject::new(self.handle.clone())
    }

    pub fn list_objects_v2(&self) -> fluent_builders::ListObjectsV2 {
        fluent_builders::ListObjectsV2::new(self.handle.clone())
    }
}

pub mod fluent_builders {
    use std::sync::Arc;

    use aws_smithy_client::SdkError;

    use crate::{
        error::{
            AbortMultipartUploadError, CreateMultipartUploadError, DeleteObjectError,
            DeleteObjectsError, GetObjectError, HeadObjectError, ListObjectsV2Error,
            PutObjectError, UploadPartError,
        },
        output::{
            AbortMultipartUploadOutput, CreateMultipartUploadOutput, DeleteObjectOutput,
            DeleteObjectsOutput, GetObjectOutput, HeadObjectOutput, ListObjectsV2Output,
            PutObjectOutput, UploadPartOutput,
        },
        types::ByteStream,
        Handle,
    };

    pub struct UploadPart {
        handle: Arc<Handle>,
        inner: crate::input::upload_part_input::Builder,
    }
    impl UploadPart {
        pub(crate) fn new(handle: Arc<Handle>) -> Self {
            Self {
                handle,
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<UploadPartOutput, SdkError<UploadPartError>> {
            todo!();
        }

        pub fn body(mut self, input: ByteStream) -> Self {
            self.inner = self.inner.body(input);
            self
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn content_length(mut self, input: i64) -> Self {
            self.inner = self.inner.content_length(input);
            self
        }

        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.key(input.into());
            self
        }

        pub fn part_number(mut self, input: i32) -> Self {
            self.inner = self.inner.part_number(input);
            self
        }

        pub fn upload_id(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.upload_id(input.into());
            self
        }
    }

    #[derive(Clone)]
    pub struct CompleteMultipartUpload {
        handle: Arc<Handle>,
        inner: crate::input::complete_multipart_upload_input::Builder,
    }
    impl CompleteMultipartUpload {
        pub(crate) fn new(handle: Arc<Handle>) -> Self {
            Self {
                handle,
                inner: Default::default(),
            }
        }

        pub async fn send(
            self,
        ) -> Result<CreateMultipartUploadOutput, SdkError<CreateMultipartUploadError>> {
            todo!();
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.key(input.into());
            self
        }
        pub fn multipart_upload(mut self, input: crate::model::CompletedMultipartUpload) -> Self {
            self.inner = self.inner.multipart_upload(input);
            self
        }

        pub fn upload_id(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.upload_id(input.into());
            self
        }
    }

    #[derive(Clone)]
    pub struct AbortMultipartUpload {
        handle: Arc<Handle>,
        inner: crate::input::abort_multipart_upload_input::Builder,
    }

    impl AbortMultipartUpload {
        pub(crate) fn new(handle: Arc<Handle>) -> Self {
            Self {
                handle,
                inner: Default::default(),
            }
        }

        pub async fn send(
            self,
        ) -> Result<AbortMultipartUploadOutput, SdkError<AbortMultipartUploadError>> {
            todo!();
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.key(input.into());
            self
        }

        pub fn upload_id(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.upload_id(input.into());
            self
        }
    }

    pub struct GetObject {
        handle: Arc<Handle>,
        inner: crate::input::get_object_input::Builder,
    }
    impl GetObject {
        pub(crate) fn new(handle: Arc<Handle>) -> Self {
            Self {
                handle,
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<GetObjectOutput, SdkError<GetObjectError>> {
            todo!();
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.key(input.into());
            self
        }

        pub fn range(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.range(input.into());
            self
        }
    }

    pub struct PutObject {
        handle: Arc<Handle>,
        inner: crate::input::put_object_input::Builder,
    }
    impl PutObject {
        pub(crate) fn new(handle: Arc<Handle>) -> Self {
            Self {
                handle,
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<PutObjectOutput, SdkError<PutObjectError>> {
            todo!();
        }

        pub fn body(mut self, input: ByteStream) -> Self {
            self.inner = self.inner.body(input);
            self
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.key(input.into());
            self
        }
    }

    pub struct DeleteObject {
        handle: Arc<Handle>,
        inner: crate::input::delete_object_input::Builder,
    }
    impl DeleteObject {
        pub(crate) fn new(handle: Arc<Handle>) -> Self {
            Self {
                handle,
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<DeleteObjectOutput, SdkError<DeleteObjectError>> {
            todo!();
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.key(input.into());
            self
        }
    }

    #[derive(Clone)]
    pub struct DeleteObjects {
        handle: Arc<Handle>,
        inner: crate::input::delete_objects_input::Builder,
    }
    impl DeleteObjects {
        pub(crate) fn new(handle: Arc<Handle>) -> Self {
            Self {
                handle,
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<DeleteObjectsOutput, SdkError<DeleteObjectsError>> {
            todo!();
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn delete(mut self, input: crate::model::Delete) -> Self {
            self.inner = self.inner.delete(input);
            self
        }
    }

    #[derive(std::clone::Clone)]
    pub struct CreateMultipartUpload {
        handle: Arc<Handle>,
        inner: crate::input::create_multipart_upload_input::Builder,
    }
    impl CreateMultipartUpload {
        pub(crate) fn new(handle: Arc<Handle>) -> Self {
            Self {
                handle,
                inner: Default::default(),
            }
        }

        pub async fn send(
            self,
        ) -> Result<CreateMultipartUploadOutput, SdkError<CreateMultipartUploadError>> {
            todo!();
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.key(input.into());
            self
        }
    }

    #[derive(Clone)]
    pub struct HeadObject {
        handle: Arc<Handle>,
        inner: crate::input::head_object_input::Builder,
    }
    impl HeadObject {
        pub(crate) fn new(handle: Arc<Handle>) -> Self {
            Self {
                handle,
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<HeadObjectOutput, SdkError<HeadObjectError>> {
            todo!();
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.key(input.into());
            self
        }
    }

    #[derive(Clone)]
    pub struct ListObjectsV2 {
        handle: Arc<Handle>,
        inner: crate::input::list_objects_v2_input::Builder,
    }
    impl ListObjectsV2 {
        pub(crate) fn new(handle: Arc<Handle>) -> Self {
            Self {
                handle,
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<ListObjectsV2Output, SdkError<ListObjectsV2Error>> {
            todo!();
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn prefix(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.prefix(input.into());
            self
        }

        pub fn continuation_token(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.continuation_token(input.into());
            self
        }
    }
}

impl Client {
    pub fn new(sdk_config: &SdkConfig) -> Self {
        Self::from_conf(sdk_config.into())
    }

    pub fn from_conf(_conf: crate::Config) -> Self {
        // let endpoint = Endpoint::connect(addr);
        // Self {
        //     handle: std::sync::Arc::new(Handle { endpoint, conf }),
        // }
        todo!();
    }
}
