pub use aws_sdk_s3::operation::complete_multipart_upload::{
    CompleteMultipartUploadError, CompleteMultipartUploadInput, CompleteMultipartUploadOutput,
};

pub mod builders {
    use super::*;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::complete_multipart_upload::builders::{
        CompleteMultipartUploadInputBuilder, CompleteMultipartUploadOutputBuilder,
    };

    impl crate::Client {
        pub fn complete_multipart_upload(&self) -> CompleteMultipartUploadFluentBuilder {
            CompleteMultipartUploadFluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    #[derive(Clone)]
    pub struct CompleteMultipartUploadFluentBuilder {
        pub(crate) client: Client,
        pub(crate) inner: CompleteMultipartUploadInputBuilder,
    }
    impl CompleteMultipartUploadFluentBuilder {
        pub async fn send(
            self,
        ) -> Result<CompleteMultipartUploadOutput, SdkError<CompleteMultipartUploadError>> {
            let input = self.inner.build().map_err(SdkError::construction_failure)?;
            let req = Request::CompletedMultipartUpload(input);
            self.client.send_request(req).await
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }
        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.key(input.into());
            self
        }
        pub fn multipart_upload(mut self, input: crate::types::CompletedMultipartUpload) -> Self {
            self.inner = self.inner.multipart_upload(input);
            self
        }
        pub fn upload_id(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.upload_id(input.into());
            self
        }
    }
}
