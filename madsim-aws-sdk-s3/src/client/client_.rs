use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct Client {}

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

    pub fn get_bucket_lifecycle_configuration(
        &self,
    ) -> fluent_builders::GetBucketLifecycleConfiguration {
        fluent_builders::GetBucketLifecycleConfiguration::new()
    }

    pub fn put_bucket_lifecycle_configuration(
        &self,
    ) -> fluent_builders::PutBucketLifecycleConfiguration {
        fluent_builders::PutBucketLifecycleConfiguration::new()
    }
}

pub mod fluent_builders {
    use crate::client::result::SdkError;
    use crate::server::error::Result as ServerResult;
    use crate::server::service::Request;
    use crate::{error::*, input::*, output::*, types::ByteStream};
    use madsim::net::{Endpoint, Payload};
    use std::net::SocketAddr;

    async fn send_aux<E: 'static>(req: Request, bucket: String) -> Result<Payload, SdkError<E>> {
        let ep = Endpoint::connect(&bucket).await?;

        let addr = bucket.parse::<SocketAddr>()?;

        let (tx, mut rx) = ep.connect1(addr).await?;

        tx.send(Box::new(req)).await?;

        Ok(rx.recv().await?)
    }

    pub struct UploadPart {
        inner: upload_part_input::Builder,
    }
    impl UploadPart {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<UploadPartOutput, SdkError<UploadPartError>> {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::UploadPart(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp.downcast::<ServerResult<UploadPartOutput>>().unwrap();
            let resp = resp?;
            Ok(resp)
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
        inner: complete_multipart_upload_input::Builder,
    }
    impl CompleteMultipartUpload {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(
            self,
        ) -> Result<CompleteMultipartUploadOutput, SdkError<CompleteMultipartUploadError>> {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::CompletedMultipartUpload(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp
                .downcast::<ServerResult<CompleteMultipartUploadOutput>>()
                .unwrap();
            let resp = resp?;
            Ok(resp)
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
        inner: abort_multipart_upload_input::Builder,
    }

    impl AbortMultipartUpload {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(
            self,
        ) -> Result<AbortMultipartUploadOutput, SdkError<AbortMultipartUploadError>> {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::AbortMultipartUpload(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp
                .downcast::<ServerResult<AbortMultipartUploadOutput>>()
                .unwrap();
            let resp = resp?;
            Ok(resp)
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
        inner: get_object_input::Builder,
    }
    impl GetObject {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<GetObjectOutput, SdkError<GetObjectError>> {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::GetObject(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp.downcast::<ServerResult<GetObjectOutput>>().unwrap();
            let resp = resp?;
            Ok(resp)
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.key(input.into());
            self
        }

        pub fn part_number(mut self, input: impl Into<i32>) -> Self {
            self.inner = self.inner.part_number(input.into());
            self
        }

        pub fn range(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.range(input.into());
            self
        }
    }

    pub struct PutObject {
        inner: put_object_input::Builder,
    }
    impl PutObject {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<PutObjectOutput, SdkError<PutObjectError>> {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::PutObject(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp.downcast::<ServerResult<PutObjectOutput>>().unwrap();
            let resp = resp?;
            Ok(resp)
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

        pub fn content_length(mut self, input: i64) -> Self {
            self.inner = self.inner.content_length(input);
            self
        }

        pub fn set_content_length(mut self, input: Option<i64>) -> Self {
            self.inner = self.inner.set_content_length(input);
            self
        }
    }

    pub struct DeleteObject {
        inner: delete_object_input::Builder,
    }
    impl DeleteObject {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<DeleteObjectOutput, SdkError<DeleteObjectError>> {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::DeleteObject(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp.downcast::<ServerResult<DeleteObjectOutput>>().unwrap();
            let resp = resp?;
            Ok(resp)
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
        inner: delete_objects_input::Builder,
    }
    impl DeleteObjects {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<DeleteObjectsOutput, SdkError<DeleteObjectsError>> {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::DeleteObjects(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp
                .downcast::<ServerResult<DeleteObjectsOutput>>()
                .unwrap();
            let resp = resp?;
            Ok(resp)
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

    #[derive(Clone)]
    pub struct CreateMultipartUpload {
        inner: create_multipart_upload_input::Builder,
    }
    impl CreateMultipartUpload {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(
            self,
        ) -> Result<CreateMultipartUploadOutput, SdkError<CreateMultipartUploadError>> {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::CreateMultipartUpload(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp
                .downcast::<ServerResult<CreateMultipartUploadOutput>>()
                .unwrap();
            let resp = resp?;
            Ok(resp)
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
        inner: head_object_input::Builder,
    }
    impl HeadObject {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<HeadObjectOutput, SdkError<HeadObjectError>> {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::HeadObject(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp.downcast::<ServerResult<HeadObjectOutput>>().unwrap();
            let resp = resp?;
            Ok(resp)
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
        inner: list_objects_v2_input::Builder,
    }
    impl ListObjectsV2 {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(self) -> Result<ListObjectsV2Output, SdkError<ListObjectsV2Error>> {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::ListObjectsV2(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp
                .downcast::<ServerResult<ListObjectsV2Output>>()
                .unwrap();
            let resp = resp?;
            Ok(resp)
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

    #[derive(Clone, Debug)]
    pub struct PutBucketLifecycleConfiguration {
        inner: put_bucket_lifecycle_configuration_input::Builder,
    }
    impl PutBucketLifecycleConfiguration {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(
            self,
        ) -> Result<
            PutBucketLifecycleConfigurationOutput,
            SdkError<PutBucketLifecycleConfigurationError>,
        > {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::PutBucketLifecycleConfiguration(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp
                .downcast::<ServerResult<PutBucketLifecycleConfigurationOutput>>()
                .unwrap();
            let resp = resp?;
            Ok(resp)
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn set_bucket(mut self, input: Option<String>) -> Self {
            self.inner = self.inner.set_bucket(input);
            self
        }

        pub fn lifecycle_configuration(
            mut self,
            input: crate::model::BucketLifecycleConfiguration,
        ) -> Self {
            self.inner = self.inner.lifecycle_configuration(input);
            self
        }

        pub fn set_lifecycle_configuration(
            mut self,
            input: Option<crate::model::BucketLifecycleConfiguration>,
        ) -> Self {
            self.inner = self.inner.set_lifecycle_configuration(input);
            self
        }

        pub fn expected_bucket_owner(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.expected_bucket_owner(input.into());
            self
        }

        pub fn set_expected_bucket_owner(mut self, input: Option<String>) -> Self {
            self.inner = self.inner.set_expected_bucket_owner(input);
            self
        }
    }

    #[derive(Clone, Debug)]
    pub struct GetBucketLifecycleConfiguration {
        inner: get_bucket_lifecycle_configuration_input::Builder,
    }
    impl GetBucketLifecycleConfiguration {
        pub(crate) fn new() -> Self {
            Self {
                inner: Default::default(),
            }
        }

        pub async fn send(
            self,
        ) -> Result<
            GetBucketLifecycleConfigurationOutput,
            SdkError<GetBucketLifecycleConfigurationError>,
        > {
            let input = self.inner.build()?;
            let bucket = input.bucket.clone();
            let req = Request::GetBucketLifecycleConfiguration(input);

            let resp = send_aux(req, bucket).await?;
            let resp = *resp
                .downcast::<ServerResult<GetBucketLifecycleConfigurationOutput>>()
                .unwrap();
            let resp = resp?;
            Ok(resp)
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn set_bucket(mut self, input: Option<String>) -> Self {
            self.inner = self.inner.set_bucket(input);
            self
        }

        pub fn expected_bucket_owner(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.expected_bucket_owner(input.into());
            self
        }

        pub fn set_expected_bucket_owner(mut self, input: Option<String>) -> Self {
            self.inner = self.inner.set_expected_bucket_owner(input);
            self
        }
    }
}

impl Client {
    pub fn new<T>(_sdk_config: T) -> Self {
        Self {}
    }

    pub fn from_conf(_conf: crate::Config) -> Self {
        Self {}
    }
}
