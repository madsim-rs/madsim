pub use aws_sdk_s3::operation::get_object::{GetObjectError, GetObjectInput, GetObjectOutput};

pub mod builders {
    use super::*;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::get_object::builders::{
        GetObjectInputBuilder, GetObjectOutputBuilder,
    };

    impl crate::Client {
        pub fn get_object(&self) -> GetObjectFluentBuilder {
            GetObjectFluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    pub struct GetObjectFluentBuilder {
        pub(super) client: Client,
        pub(super) inner: GetObjectInputBuilder,
    }
    impl GetObjectFluentBuilder {
        pub async fn send(self) -> Result<GetObjectOutput, SdkError<GetObjectError>> {
            let input = self.inner.build().map_err(SdkError::construction_failure)?;
            let req = Request::GetObject(input);
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

        pub fn part_number(mut self, input: impl Into<i32>) -> Self {
            self.inner = self.inner.part_number(input.into());
            self
        }

        pub fn range(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.range(input.into());
            self
        }
    }
}
