pub use aws_sdk_s3::operation::delete_objects::{
    DeleteObjectsError, DeleteObjectsInput, DeleteObjectsOutput,
};

pub mod builders {
    use super::*;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::delete_objects::builders::{
        DeleteObjectsInputBuilder, DeleteObjectsOutputBuilder,
    };

    impl crate::Client {
        pub fn delete_objects(&self) -> DeleteObjectsFluentBuilder {
            DeleteObjectsFluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    pub struct DeleteObjectsFluentBuilder {
        pub(super) client: Client,
        pub(super) inner: DeleteObjectsInputBuilder,
    }
    impl DeleteObjectsFluentBuilder {
        pub async fn send(self) -> Result<DeleteObjectsOutput, SdkError<DeleteObjectsError>> {
            let input = self.inner.build().map_err(SdkError::construction_failure)?;
            let req = Request::DeleteObjects(input);
            self.client.send_request(req).await
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn delete(mut self, input: crate::types::Delete) -> Self {
            self.inner = self.inner.delete(input);
            self
        }
    }
}
