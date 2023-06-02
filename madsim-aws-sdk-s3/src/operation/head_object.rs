pub use aws_sdk_s3::operation::head_object::{HeadObjectError, HeadObjectInput, HeadObjectOutput};

pub mod builders {
    use super::*;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::head_object::builders::{
        HeadObjectInputBuilder, HeadObjectOutputBuilder,
    };

    impl crate::Client {
        pub fn head_object(&self) -> HeadObjectFluentBuilder {
            HeadObjectFluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    #[derive(Clone)]
    pub struct HeadObjectFluentBuilder {
        pub(super) client: Client,
        pub(super) inner: HeadObjectInputBuilder,
    }
    impl HeadObjectFluentBuilder {
        pub async fn send(self) -> Result<HeadObjectOutput, SdkError<HeadObjectError>> {
            let input = self.inner.build().map_err(SdkError::construction_failure)?;
            let req = Request::HeadObject(input);
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
