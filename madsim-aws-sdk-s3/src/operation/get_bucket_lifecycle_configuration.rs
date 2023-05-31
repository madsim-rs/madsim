pub use aws_sdk_s3::operation::get_bucket_lifecycle_configuration::{
    GetBucketLifecycleConfigurationError, GetBucketLifecycleConfigurationInput,
    GetBucketLifecycleConfigurationOutput,
};

pub mod builders {
    use super::*;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::get_bucket_lifecycle_configuration::builders::{
        GetBucketLifecycleConfigurationInputBuilder, GetBucketLifecycleConfigurationOutputBuilder,
    };

    impl crate::Client {
        pub fn get_bucket_lifecycle_configuration(
            &self,
        ) -> GetBucketLifecycleConfigurationFluentBuilder {
            GetBucketLifecycleConfigurationFluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct GetBucketLifecycleConfigurationFluentBuilder {
        pub(super) client: Client,
        pub(super) inner: GetBucketLifecycleConfigurationInputBuilder,
    }
    impl GetBucketLifecycleConfigurationFluentBuilder {
        pub async fn send(
            self,
        ) -> Result<
            GetBucketLifecycleConfigurationOutput,
            SdkError<GetBucketLifecycleConfigurationError>,
        > {
            let input = self.inner.build().map_err(SdkError::construction_failure)?;
            let req = Request::GetBucketLifecycleConfiguration(input);
            self.client.send_request(req).await
        }

        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.bucket(input.into());
            self
        }

        pub fn set_bucket(mut self, input: Option<String>) -> Self {
            self.inner = self.inner.set_bucket(input);
            self
        }

        pub fn expected_bucket_owner(mut self, input: impl Into<String>) -> Self {
            self.inner = self.inner.expected_bucket_owner(input.into());
            self
        }

        pub fn set_expected_bucket_owner(mut self, input: Option<String>) -> Self {
            self.inner = self.inner.set_expected_bucket_owner(input);
            self
        }
    }
}
