pub use aws_sdk_s3::operation::put_object::{PutObjectError, PutObjectInput, PutObjectOutput};

pub mod builders {
    use super::*;
    use crate::primitives::ByteStream;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::put_object::builders::{
        PutObjectInputBuilder, PutObjectOutputBuilder,
    };

    impl crate::Client {
        pub fn put_object(&self) -> PutObjectFluentBuilder {
            PutObjectFluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    pub struct PutObjectFluentBuilder {
        pub(super) client: Client,
        pub(super) inner: PutObjectInputBuilder,
    }
    impl PutObjectFluentBuilder {
        pub async fn send(self) -> Result<PutObjectOutput, SdkError<PutObjectError>> {
            let mut input = self.inner.build().map_err(SdkError::construction_failure)?;
            // collect the body stream into a single buffer
            input.body = input
                .body
                .collect()
                .await
                .map_err(SdkError::construction_failure)?
                .into_bytes()
                .into();
            let req = Request::PutObject(input);
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
}
