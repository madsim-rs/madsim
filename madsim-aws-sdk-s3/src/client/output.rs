use crate::types::ByteStream;
use std::fmt::{Debug, Formatter, Result as FmtResult};

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct UploadPartOutput {
    pub e_tag: Option<String>,
}
impl UploadPartOutput {
    pub fn e_tag(&self) -> Option<&str> {
        self.e_tag.as_deref()
    }
}
impl Debug for UploadPartOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("UploadPartOutput");
        formatter.field("e_tag", &self.e_tag);
        formatter.finish()
    }
}
pub mod upload_part_output {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) e_tag: Option<String>,
    }
    impl Builder {
        pub fn e_tag(mut self, input: impl Into<String>) -> Self {
            self.e_tag = Some(input.into());
            self
        }

        pub fn set_e_tag(mut self, input: Option<String>) -> Self {
            self.e_tag = input;
            self
        }

        pub fn build(self) -> crate::output::UploadPartOutput {
            crate::output::UploadPartOutput { e_tag: self.e_tag }
        }
    }
}
impl UploadPartOutput {
    pub fn builder() -> crate::output::upload_part_output::Builder {
        crate::output::upload_part_output::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct CreateMultipartUploadOutput {
    pub upload_id: Option<String>,
}
impl CreateMultipartUploadOutput {
    pub fn upload_id(&self) -> Option<&str> {
        self.upload_id.as_deref()
    }
}
impl Debug for CreateMultipartUploadOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("CreateMultipartUploadOutput");
        formatter.field("upload_id", &self.upload_id);
        formatter.finish()
    }
}
pub mod create_multipart_output {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) upload_id: Option<String>,
    }
    impl Builder {
        pub fn upload_id(mut self, input: impl Into<String>) -> Self {
            self.upload_id = Some(input.into());
            self
        }

        pub fn set_upload_id(mut self, input: Option<String>) -> Self {
            self.upload_id = input;
            self
        }

        pub fn build(self) -> crate::output::CreateMultipartUploadOutput {
            crate::output::CreateMultipartUploadOutput {
                upload_id: self.upload_id,
            }
        }
    }
}
impl CreateMultipartUploadOutput {
    pub fn builder() -> crate::output::create_multipart_output::Builder {
        crate::output::create_multipart_output::Builder::default()
    }
}

#[non_exhaustive]
pub struct GetObjectOutput {
    pub body: ByteStream,
}
impl GetObjectOutput {
    pub fn body(&self) -> &ByteStream {
        &self.body
    }
}
impl Debug for GetObjectOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("GetObjectOutput");
        formatter.field("body", &self.body);
        formatter.finish()
    }
}
pub mod get_object_output {

    #[derive(Default, Debug)]
    pub struct Builder {
        pub(crate) body: Option<crate::types::ByteStream>,
    }
    impl Builder {
        pub fn body(mut self, input: crate::types::ByteStream) -> Self {
            self.body = Some(input);
            self
        }

        pub fn set_body(mut self, input: Option<crate::types::ByteStream>) -> Self {
            self.body = input;
            self
        }

        pub fn build(self) -> crate::output::GetObjectOutput {
            crate::output::GetObjectOutput {
                body: self.body.unwrap_or_default(),
            }
        }
    }
}
impl GetObjectOutput {
    pub fn builder() -> crate::output::get_object_output::Builder {
        crate::output::get_object_output::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq)]
pub struct HeadObjectOutput {
    pub last_modified: Option<crate::types::DateTime>,
    pub content_length: i64,
}
impl HeadObjectOutput {
    pub fn last_modified(&self) -> Option<&aws_smithy_types::DateTime> {
        self.last_modified.as_ref()
    }

    pub fn content_length(&self) -> i64 {
        self.content_length
    }
}
impl Debug for HeadObjectOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("HeadObjectOutput");
        formatter.field("last_modified", &self.last_modified);
        formatter.field("content_length", &self.content_length);
        formatter.finish()
    }
}
pub mod head_object_output {

    #[derive(Default, Clone, PartialEq, Debug)]
    pub struct Builder {
        pub(crate) content_length: Option<i64>,
        pub(crate) last_modified: Option<aws_smithy_types::DateTime>,
    }
    impl Builder {
        pub fn last_modified(mut self, input: aws_smithy_types::DateTime) -> Self {
            self.last_modified = Some(input);
            self
        }

        pub fn set_last_modified(mut self, input: Option<aws_smithy_types::DateTime>) -> Self {
            self.last_modified = input;
            self
        }

        pub fn content_length(mut self, input: i64) -> Self {
            self.content_length = Some(input);
            self
        }

        pub fn set_content_length(mut self, input: Option<i64>) -> Self {
            self.content_length = input;
            self
        }

        pub fn build(self) -> crate::output::HeadObjectOutput {
            crate::output::HeadObjectOutput {
                last_modified: self.last_modified,
                content_length: self.content_length.unwrap_or_default(),
            }
        }
    }
}
impl HeadObjectOutput {
    pub fn builder() -> crate::output::head_object_output::Builder {
        crate::output::head_object_output::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq)]
pub struct ListObjectsV2Output {
    pub is_truncated: bool,
    pub contents: Option<Vec<crate::model::Object>>,
    pub next_continuation_token: Option<String>,
}
impl ListObjectsV2Output {
    pub fn is_truncated(&self) -> bool {
        self.is_truncated
    }
    pub fn contents(&self) -> Option<&[crate::model::Object]> {
        self.contents.as_deref()
    }
    pub fn next_continuation_token(&self) -> Option<&str> {
        self.next_continuation_token.as_deref()
    }
}
impl Debug for ListObjectsV2Output {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("ListObjectsV2Output");
        formatter.field("is_truncated", &self.is_truncated);
        formatter.field("contents", &self.contents);
        formatter.field("next_continuation_token", &self.next_continuation_token);
        formatter.finish()
    }
}

pub mod list_objects_v2_output {

    #[derive(Default, Clone, PartialEq, Debug)]
    pub struct Builder {
        pub(crate) is_truncated: Option<bool>,
        pub(crate) contents: Option<Vec<crate::model::Object>>,
        pub(crate) next_continuation_token: Option<String>,
    }
    impl Builder {
        pub fn is_truncated(mut self, input: bool) -> Self {
            self.is_truncated = Some(input);
            self
        }

        pub fn set_is_truncated(mut self, input: Option<bool>) -> Self {
            self.is_truncated = input;
            self
        }

        pub fn contents(mut self, input: crate::model::Object) -> Self {
            let mut v = self.contents.unwrap_or_default();
            v.push(input);
            self.contents = Some(v);
            self
        }

        pub fn set_contents(mut self, input: Option<Vec<crate::model::Object>>) -> Self {
            self.contents = input;
            self
        }

        pub fn next_continuation_token(mut self, input: impl Into<String>) -> Self {
            self.next_continuation_token = Some(input.into());
            self
        }

        pub fn set_next_continuation_token(mut self, input: Option<String>) -> Self {
            self.next_continuation_token = input;
            self
        }

        pub fn build(self) -> crate::output::ListObjectsV2Output {
            crate::output::ListObjectsV2Output {
                is_truncated: self.is_truncated.unwrap_or_default(),
                contents: self.contents,
                next_continuation_token: self.next_continuation_token,
            }
        }
    }
}
impl ListObjectsV2Output {
    pub fn builder() -> crate::output::list_objects_v2_output::Builder {
        crate::output::list_objects_v2_output::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct DeleteObjectsOutput {
    pub errors: Option<Vec<crate::model::Error>>,
}
impl DeleteObjectsOutput {
    pub fn errors(&self) -> Option<&[crate::model::Error]> {
        self.errors.as_deref()
    }
}
impl Debug for DeleteObjectsOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("DeleteObjectsOutput");
        formatter.field("errors", &self.errors);
        formatter.finish()
    }
}

pub mod delete_objects_output {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) errors: Option<Vec<crate::model::Error>>,
    }
    impl Builder {
        pub fn errors(mut self, input: crate::model::Error) -> Self {
            let mut v = self.errors.unwrap_or_default();
            v.push(input);
            self.errors = Some(v);
            self
        }

        pub fn set_errors(mut self, input: Option<Vec<crate::model::Error>>) -> Self {
            self.errors = input;
            self
        }

        pub fn build(self) -> crate::output::DeleteObjectsOutput {
            crate::output::DeleteObjectsOutput {
                errors: self.errors,
            }
        }
    }
}
impl DeleteObjectsOutput {
    pub fn builder() -> crate::output::delete_objects_output::Builder {
        crate::output::delete_objects_output::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct CompleteMultipartUploadOutput {}
impl CompleteMultipartUploadOutput {}
impl Debug for CompleteMultipartUploadOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("CompleteMultipartUploadOutput");
        formatter.finish()
    }
}
pub mod complete_multipart_upload_output {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {}
    impl Builder {
        pub fn build(self) -> crate::output::CompleteMultipartUploadOutput {
            crate::output::CompleteMultipartUploadOutput {}
        }
    }
}
impl CompleteMultipartUploadOutput {
    pub fn builder() -> crate::output::complete_multipart_upload_output::Builder {
        crate::output::complete_multipart_upload_output::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct AbortMultipartUploadOutput {}
impl AbortMultipartUploadOutput {}
impl Debug for AbortMultipartUploadOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("AbortMultipartUploadOutput");
        formatter.finish()
    }
}
pub mod abort_multipart_upload_output {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {}
    impl Builder {
        pub fn build(self) -> crate::output::AbortMultipartUploadOutput {
            crate::output::AbortMultipartUploadOutput {}
        }
    }
}
impl AbortMultipartUploadOutput {
    pub fn builder() -> crate::output::abort_multipart_upload_output::Builder {
        crate::output::abort_multipart_upload_output::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct PutObjectOutput {}
impl PutObjectOutput {}
impl Debug for PutObjectOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("PutObjectOutput");
        formatter.finish()
    }
}
pub mod put_object_output {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {}
    impl Builder {
        pub fn build(self) -> crate::output::PutObjectOutput {
            crate::output::PutObjectOutput {}
        }
    }
}
impl PutObjectOutput {
    pub fn builder() -> crate::output::put_object_output::Builder {
        crate::output::put_object_output::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct DeleteObjectOutput {}
impl DeleteObjectOutput {}
impl Debug for DeleteObjectOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("DeleteObjectOutput");
        formatter.finish()
    }
}
pub mod delete_object_output {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {}
    impl Builder {
        pub fn build(self) -> crate::output::DeleteObjectOutput {
            crate::output::DeleteObjectOutput {}
        }
    }
}
impl DeleteObjectOutput {
    pub fn builder() -> crate::output::delete_object_output::Builder {
        crate::output::delete_object_output::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq)]
pub struct GetBucketLifecycleConfigurationOutput {
    pub rules: Option<Vec<crate::model::LifecycleRule>>,
}
impl GetBucketLifecycleConfigurationOutput {
    pub fn rules(&self) -> Option<&[crate::model::LifecycleRule]> {
        self.rules.as_deref()
    }
}
impl Debug for GetBucketLifecycleConfigurationOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("GetBucketLifecycleConfigurationOutput");
        formatter.field("rules", &self.rules);
        formatter.finish()
    }
}
pub mod get_bucket_lifecycle_configuration_output {

    #[derive(Default, Clone, PartialEq, Debug)]
    pub struct Builder {
        pub(crate) rules: Option<Vec<crate::model::LifecycleRule>>,
    }
    impl Builder {
        pub fn rules(mut self, input: crate::model::LifecycleRule) -> Self {
            let mut v = self.rules.unwrap_or_default();
            v.push(input);
            self.rules = Some(v);
            self
        }
        pub fn set_rules(mut self, input: Option<Vec<crate::model::LifecycleRule>>) -> Self {
            self.rules = input;
            self
        }
        pub fn build(self) -> crate::output::GetBucketLifecycleConfigurationOutput {
            crate::output::GetBucketLifecycleConfigurationOutput { rules: self.rules }
        }
    }
}
impl GetBucketLifecycleConfigurationOutput {
    pub fn builder() -> crate::output::get_bucket_lifecycle_configuration_output::Builder {
        crate::output::get_bucket_lifecycle_configuration_output::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct PutBucketLifecycleConfigurationOutput {}
impl Debug for PutBucketLifecycleConfigurationOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("PutBucketLifecycleConfigurationOutput");
        formatter.finish()
    }
}
pub mod put_bucket_lifecycle_configuration_output {

    #[derive(Default, Clone, PartialEq, Eq, Debug)]
    pub struct Builder {}
    impl Builder {
        pub fn build(self) -> crate::output::PutBucketLifecycleConfigurationOutput {
            crate::output::PutBucketLifecycleConfigurationOutput {}
        }
    }
}
impl PutBucketLifecycleConfigurationOutput {
    pub fn builder() -> crate::output::put_bucket_lifecycle_configuration_output::Builder {
        crate::output::put_bucket_lifecycle_configuration_output::Builder::default()
    }
}
