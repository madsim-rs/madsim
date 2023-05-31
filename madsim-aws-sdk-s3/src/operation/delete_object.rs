pub use aws_sdk_s3::operation::delete_object::{
    DeleteObjectError, DeleteObjectInput, DeleteObjectOutput,
};

pub mod builders {
    use super::*;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::delete_object::builders::{
        DeleteObjectInputBuilder, DeleteObjectOutputBuilder,
    };

    impl crate::Client {
        pub fn delete_object(&self) -> DeleteObjectFluentBuilder {
            DeleteObjectFluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    pub struct DeleteObjectFluentBuilder {
        pub(super) client: Client,
        pub(super) inner: DeleteObjectInputBuilder,
    }
    impl DeleteObjectFluentBuilder {
        pub async fn send(self) -> Result<DeleteObjectOutput, SdkError<DeleteObjectError>> {
            let input = self.inner.build().map_err(SdkError::construction_failure)?;
            let req = Request::DeleteObject(input);
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
