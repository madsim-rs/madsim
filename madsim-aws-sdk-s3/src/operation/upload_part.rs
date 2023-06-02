pub use aws_sdk_s3::operation::upload_part::{UploadPartError, UploadPartInput, UploadPartOutput};

pub mod builders {
    use super::*;
    use crate::primitives::ByteStream;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::upload_part::builders::{
        UploadPartInputBuilder, UploadPartOutputBuilder,
    };

    impl crate::Client {
        pub fn upload_part(&self) -> UploadPartFluentBuilder {
            UploadPartFluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    #[derive(Debug)]
    pub struct UploadPartFluentBuilder {
        pub(crate) client: Client,
        pub(crate) inner: UploadPartInputBuilder,
    }

    impl UploadPartFluentBuilder {
        pub async fn send(self) -> Result<UploadPartOutput, SdkError<UploadPartError>> {
            let mut input = self.inner.build().map_err(SdkError::construction_failure)?;
            // collect the body stream into a single buffer
            input.body = input
                .body
                .collect()
                .await
                .map_err(SdkError::construction_failure)?
                .into_bytes()
                .into();
            let req = Request::UploadPart(input);
            self.client.send_request(req).await
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
}
