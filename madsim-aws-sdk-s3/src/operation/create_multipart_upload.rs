pub use aws_sdk_s3::operation::create_multipart_upload::{
    CreateMultipartUploadError, CreateMultipartUploadInput, CreateMultipartUploadOutput,
};

pub mod builders {
    use super::*;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::create_multipart_upload::builders::{
        CreateMultipartUploadInputBuilder, CreateMultipartUploadOutputBuilder,
    };

    impl crate::Client {
        pub fn create_multipart_upload(&self) -> CreateMultipartUploadFluentBuilder {
            CreateMultipartUploadFluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    #[derive(Clone)]
    pub struct CreateMultipartUploadFluentBuilder {
        pub(super) client: Client,
        pub(super) inner: CreateMultipartUploadInputBuilder,
    }
    impl CreateMultipartUploadFluentBuilder {
        pub async fn send(
            self,
        ) -> Result<CreateMultipartUploadOutput, SdkError<CreateMultipartUploadError>> {
            let input = self.inner.build().map_err(SdkError::construction_failure)?;
            let req = Request::CreateMultipartUpload(input);
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
    }
}
