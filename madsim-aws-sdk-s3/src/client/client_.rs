use super::Config;

use aws_types::SdkConfig;
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Client {
    config: Arc<Config>,
}

impl Client {
    pub fn create_multipart_upload(&self) -> fluent_builders::CreateMultipartUpload {
        fluent_builders::CreateMultipartUpload {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }

    pub fn upload_part(&self) -> fluent_builders::UploadPart {
        fluent_builders::UploadPart {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }

    pub fn complete_multipart_upload(&self) -> fluent_builders::CompleteMultipartUpload {
        fluent_builders::CompleteMultipartUpload {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }

    pub fn abort_multipart_upload(&self) -> fluent_builders::AbortMultipartUpload {
        fluent_builders::AbortMultipartUpload {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }

    pub fn get_object(&self) -> fluent_builders::GetObject {
        fluent_builders::GetObject {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }

    pub fn put_object(&self) -> fluent_builders::PutObject {
        fluent_builders::PutObject {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }

    pub fn delete_object(&self) -> fluent_builders::DeleteObject {
        fluent_builders::DeleteObject {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }

    pub fn delete_objects(&self) -> fluent_builders::DeleteObjects {
        fluent_builders::DeleteObjects {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }

    pub fn head_object(&self) -> fluent_builders::HeadObject {
        fluent_builders::HeadObject {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }

    pub fn list_objects_v2(&self) -> fluent_builders::ListObjectsV2 {
        fluent_builders::ListObjectsV2 {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }

    pub fn get_bucket_lifecycle_configuration(
        &self,
    ) -> fluent_builders::GetBucketLifecycleConfiguration {
        fluent_builders::GetBucketLifecycleConfiguration {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }

    pub fn put_bucket_lifecycle_configuration(
        &self,
    ) -> fluent_builders::PutBucketLifecycleConfiguration {
        fluent_builders::PutBucketLifecycleConfiguration {
            config: self.config.clone(),
            inner: Default::default(),
        }
    }
}

pub mod fluent_builders {
    use super::*;
    use crate::server::service::Request;
    use crate::{error::*, input::*, output::*, types::ByteStream};
    use aws_sdk_s3::types::SdkError;
    use madsim::net::Endpoint;
    use std::net::SocketAddr;

    async fn send_aux<O: 'static, E: 'static>(
        cfg: &Config,
        req: Request,
    ) -> Result<O, SdkError<E>> {
        let addr = cfg
            .endpoint
            .uri()
            .authority()
            .expect("invalid URI")
            .as_str()
            .parse::<SocketAddr>()
            .unwrap();
        let ep = Endpoint::connect(addr).await.map_err(io_err)?;
        let (tx, mut rx) = ep.connect1(addr).await.map_err(io_err)?;
        tx.send(Box::new(req)).await.map_err(io_err)?;
        let resp = rx.recv().await.map_err(io_err)?;
        let resp = *resp.downcast::<Result<O, E>>().expect("failed to downcast");
        resp.map_err(|e| SdkError::ServiceError { err: e, raw: raw() })
    }

    pub struct UploadPart {
        pub(super) config: Arc<Config>,
        pub(super) inner: upload_part_input::Builder,
    }
    impl UploadPart {
        pub async fn send(self) -> Result<UploadPartOutput, SdkError<UploadPartError>> {
            let mut input = self.inner.build().map_err(build_err)?;
            input.collect_body().await.map_err(build_err)?;
            let req = Request::UploadPart(input);
            send_aux(&self.config, req).await
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
        pub(super) config: Arc<Config>,
        pub(super) inner: complete_multipart_upload_input::Builder,
    }
    impl CompleteMultipartUpload {
        pub async fn send(
            self,
        ) -> Result<CompleteMultipartUploadOutput, SdkError<CompleteMultipartUploadError>> {
            let input = self.inner.build().map_err(build_err)?;
            let req = Request::CompletedMultipartUpload(input);
            send_aux(&self.config, req).await
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
        pub(super) config: Arc<Config>,
        pub(super) inner: abort_multipart_upload_input::Builder,
    }

    impl AbortMultipartUpload {
        pub async fn send(
            self,
        ) -> Result<AbortMultipartUploadOutput, SdkError<AbortMultipartUploadError>> {
            let input = self.inner.build().map_err(build_err)?;
            let req = Request::AbortMultipartUpload(input);
            send_aux(&self.config, req).await
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
        pub(super) config: Arc<Config>,
        pub(super) inner: get_object_input::Builder,
    }
    impl GetObject {
        pub async fn send(self) -> Result<GetObjectOutput, SdkError<GetObjectError>> {
            let input = self.inner.build().map_err(build_err)?;
            let req = Request::GetObject(input);
            send_aux(&self.config, req).await
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
        pub(super) config: Arc<Config>,
        pub(super) inner: put_object_input::Builder,
    }
    impl PutObject {
        pub async fn send(self) -> Result<PutObjectOutput, SdkError<PutObjectError>> {
            let mut input = self.inner.build().map_err(build_err)?;
            input.collect_body().await.map_err(build_err)?;
            let req = Request::PutObject(input);
            send_aux(&self.config, req).await
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
        pub(super) config: Arc<Config>,
        pub(super) inner: delete_object_input::Builder,
    }
    impl DeleteObject {
        pub async fn send(self) -> Result<DeleteObjectOutput, SdkError<DeleteObjectError>> {
            let input = self.inner.build().map_err(build_err)?;
            let req = Request::DeleteObject(input);
            send_aux(&self.config, req).await
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
        pub(super) config: Arc<Config>,
        pub(super) inner: delete_objects_input::Builder,
    }
    impl DeleteObjects {
        pub async fn send(self) -> Result<DeleteObjectsOutput, SdkError<DeleteObjectsError>> {
            let input = self.inner.build().map_err(build_err)?;
            let req = Request::DeleteObjects(input);
            send_aux(&self.config, req).await
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
        pub(super) config: Arc<Config>,
        pub(super) inner: create_multipart_upload_input::Builder,
    }
    impl CreateMultipartUpload {
        pub async fn send(
            self,
        ) -> Result<CreateMultipartUploadOutput, SdkError<CreateMultipartUploadError>> {
            let input = self.inner.build().map_err(build_err)?;
            let req = Request::CreateMultipartUpload(input);
            send_aux(&self.config, req).await
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
        pub(super) config: Arc<Config>,
        pub(super) inner: head_object_input::Builder,
    }
    impl HeadObject {
        pub async fn send(self) -> Result<HeadObjectOutput, SdkError<HeadObjectError>> {
            let input = self.inner.build().map_err(build_err)?;
            let req = Request::HeadObject(input);
            send_aux(&self.config, req).await
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
        pub(super) config: Arc<Config>,
        pub(super) inner: list_objects_v2_input::Builder,
    }
    impl ListObjectsV2 {
        pub async fn send(self) -> Result<ListObjectsV2Output, SdkError<ListObjectsV2Error>> {
            let input = self.inner.build().map_err(build_err)?;
            let req = Request::ListObjectsV2(input);
            send_aux(&self.config, req).await
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
        pub(super) config: Arc<Config>,
        pub(super) inner: put_bucket_lifecycle_configuration_input::Builder,
    }
    impl PutBucketLifecycleConfiguration {
        pub async fn send(
            self,
        ) -> Result<
            PutBucketLifecycleConfigurationOutput,
            SdkError<PutBucketLifecycleConfigurationError>,
        > {
            let input = self.inner.build().map_err(build_err)?;
            let req = Request::PutBucketLifecycleConfiguration(input);
            send_aux(&self.config, req).await
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
        pub(super) config: Arc<Config>,
        pub(super) inner: get_bucket_lifecycle_configuration_input::Builder,
    }
    impl GetBucketLifecycleConfiguration {
        pub async fn send(
            self,
        ) -> Result<
            GetBucketLifecycleConfigurationOutput,
            SdkError<GetBucketLifecycleConfigurationError>,
        > {
            let input = self.inner.build().map_err(build_err)?;
            let req = Request::GetBucketLifecycleConfiguration(input);
            send_aux(&self.config, req).await
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

    /// Returns an empty raw response.
    fn raw() -> aws_smithy_http::operation::Response {
        aws_smithy_http::operation::Response::new(http::response::Response::new(
            aws_smithy_http::body::SdkBody::empty(),
        ))
    }

    /// Wrap a build error to SkdError.
    fn build_err<E>(e: impl std::error::Error + Send + Sync + 'static) -> SdkError<E> {
        SdkError::ConstructionFailure(Box::new(e))
    }

    /// Wrap an IO error to SkdError.
    fn io_err<E>(e: std::io::Error) -> SdkError<E> {
        use aws_smithy_http::result::ConnectorError;
        SdkError::DispatchFailure(ConnectorError::io(Box::new(e)))
    }
}

impl Client {
    pub fn new(sdk_config: &SdkConfig) -> Self {
        Self::from_conf(sdk_config.into())
    }

    pub fn from_conf(conf: crate::Config) -> Self {
        tracing::debug!(?conf, "new client");
        Self {
            config: Arc::new(conf),
        }
    }
}
