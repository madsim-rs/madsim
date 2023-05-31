pub use aws_sdk_s3::operation::put_bucket_lifecycle_configuration::{
    PutBucketLifecycleConfigurationError, PutBucketLifecycleConfigurationInput,
    PutBucketLifecycleConfigurationOutput,
};

pub mod builders {
    use super::*;
    use crate::server::service::Request;
    use crate::{error::SdkError, Client};

    pub use aws_sdk_s3::operation::put_bucket_lifecycle_configuration::builders::{
        PutBucketLifecycleConfigurationInputBuilder, PutBucketLifecycleConfigurationOutputBuilder,
    };

    impl crate::Client {
        pub fn put_bucket_lifecycle_configuration(
            &self,
        ) -> PutBucketLifecycleConfigurationFluentBuilder {
            PutBucketLifecycleConfigurationFluentBuilder {
                client: self.clone(),
                inner: Default::default(),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct PutBucketLifecycleConfigurationFluentBuilder {
        pub(super) client: Client,
        pub(super) inner: PutBucketLifecycleConfigurationInputBuilder,
    }
    impl PutBucketLifecycleConfigurationFluentBuilder {
        pub async fn send(
            self,
        ) -> Result<
            PutBucketLifecycleConfigurationOutput,
            SdkError<PutBucketLifecycleConfigurationError>,
        > {
            let input = self.inner.build().map_err(SdkError::construction_failure)?;
            let req = Request::PutBucketLifecycleConfiguration(input);
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

        pub fn lifecycle_configuration(
            mut self,
            input: crate::types::BucketLifecycleConfiguration,
        ) -> Self {
            self.inner = self.inner.lifecycle_configuration(input);
            self
        }

        pub fn set_lifecycle_configuration(
            mut self,
            input: Option<crate::types::BucketLifecycleConfiguration>,
        ) -> Self {
            self.inner = self.inner.set_lifecycle_configuration(input);
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
