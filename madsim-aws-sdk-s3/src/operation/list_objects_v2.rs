pub use aws_sdk_s3::operation::list_objects_v2::{
    ListObjectsV2Error, ListObjectsV2Input, ListObjectsV2Output,
};

pub mod builders {
    use super::*;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::list_objects_v2::builders::{
        ListObjectsV2InputBuilder, ListObjectsV2OutputBuilder,
    };

    impl crate::Client {
        pub fn list_objects_v2(&self) -> ListObjectsV2FluentBuilder {
            ListObjectsV2FluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    #[derive(Clone)]
    pub struct ListObjectsV2FluentBuilder {
        pub(super) client: Client,
        pub(super) inner: ListObjectsV2InputBuilder,
    }
    impl ListObjectsV2FluentBuilder {
        pub async fn send(self) -> Result<ListObjectsV2Output, SdkError<ListObjectsV2Error>> {
            let input = self.inner.build().map_err(SdkError::construction_failure)?;
            let req = Request::ListObjectsV2(input);
            self.client.send_request(req).await
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
}
