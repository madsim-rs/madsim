pub use aws_sdk_s3::operation::abort_multipart_upload::{
    AbortMultipartUploadError, AbortMultipartUploadInput, AbortMultipartUploadOutput,
};

pub mod builders {
    use super::*;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::abort_multipart_upload::builders::{
        AbortMultipartUploadInputBuilder, AbortMultipartUploadOutputBuilder,
    };

    impl crate::Client {
        pub fn abort_multipart_upload(&self) -> AbortMultipartUploadFluentBuilder {
            AbortMultipartUploadFluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    #[derive(Clone)]
    pub struct AbortMultipartUploadFluentBuilder {
        pub(super) client: Client,
        pub(super) inner: AbortMultipartUploadInputBuilder,
    }

    impl AbortMultipartUploadFluentBuilder {
        pub async fn send(
            self,
        ) -> Result<AbortMultipartUploadOutput, SdkError<AbortMultipartUploadError>> {
            let input = self.inner.build().map_err(SdkError::construction_failure)?;
            let req = Request::AbortMultipartUpload(input);
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

        pub fn upload_id(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.upload_id(input.into());
            self
        }
    }
}
