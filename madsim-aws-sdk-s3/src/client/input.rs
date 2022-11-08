pub mod upload_part_input {
    use super::UploadPartInput;

    #[derive(Default, Debug)]
    pub struct Builder {
        pub(crate) body: Option<crate::types::ByteStream>,
        pub(crate) bucket: Option<String>,
        pub(crate) content_length: Option<i64>,
        pub(crate) key: Option<String>,
        pub(crate) part_number: Option<i32>,
        pub(crate) upload_id: Option<String>,
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
        pub fn bucket(mut self, input: impl Into<String>) -> Self {
            self.bucket = Some(input.into());
            self
        }
        pub fn set_bucket(mut self, input: Option<String>) -> Self {
            self.bucket = input;
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
        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: Option<String>) -> Self {
            self.key = input;
            self
        }
        pub fn part_number(mut self, input: i32) -> Self {
            self.part_number = Some(input);
            self
        }
        pub fn set_part_number(mut self, input: Option<i32>) -> Self {
            self.part_number = input;
            self
        }
        pub fn upload_id(mut self, input: impl Into<String>) -> Self {
            self.upload_id = Some(input.into());
            self
        }
        pub fn set_upload_id(mut self, input: Option<String>) -> Self {
            self.upload_id = input;
            self
        }
        pub fn build(self) -> Result<UploadPartInput, aws_smithy_http::operation::BuildError> {
            Ok(UploadPartInput {
                body: self.body.unwrap_or_default(),
                bucket: self.bucket,
                content_length: self.content_length.unwrap_or_default(),
                key: self.key,
                part_number: self.part_number.unwrap_or_default(),
                upload_id: self.upload_id,
            })
        }
    }
}
impl UploadPartInput {
    pub fn builder() -> crate::input::upload_part_input::Builder {
        crate::input::upload_part_input::Builder::default()
    }
}

pub mod complete_multipart_upload_input {

    #[derive(
        std::default::Default, std::clone::Clone, std::cmp::PartialEq, Eq, std::fmt::Debug,
    )]
    pub struct Builder {
        pub(crate) bucket: std::option::Option<std::string::String>,
        pub(crate) key: std::option::Option<std::string::String>,
        pub(crate) multipart_upload: std::option::Option<crate::model::CompletedMultipartUpload>,
        pub(crate) upload_id: std::option::Option<std::string::String>,
    }
    impl Builder {
        pub fn bucket(mut self, input: impl Into<std::string::String>) -> Self {
            self.bucket = Some(input.into());
            self
        }
        pub fn set_bucket(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.bucket = input;
            self
        }
        pub fn key(mut self, input: impl Into<std::string::String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.key = input;
            self
        }
        pub fn multipart_upload(mut self, input: crate::model::CompletedMultipartUpload) -> Self {
            self.multipart_upload = Some(input);
            self
        }
        pub fn set_multipart_upload(
            mut self,
            input: std::option::Option<crate::model::CompletedMultipartUpload>,
        ) -> Self {
            self.multipart_upload = input;
            self
        }
        pub fn upload_id(mut self, input: impl Into<std::string::String>) -> Self {
            self.upload_id = Some(input.into());
            self
        }
        pub fn set_upload_id(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.upload_id = input;
            self
        }
        pub fn build(
            self,
        ) -> Result<
            crate::input::CompleteMultipartUploadInput,
            aws_smithy_http::operation::BuildError,
        > {
            Ok(crate::input::CompleteMultipartUploadInput {
                bucket: self.bucket,
                key: self.key,
                multipart_upload: self.multipart_upload,
                upload_id: self.upload_id,
            })
        }
    }
}
impl CompleteMultipartUploadInput {
    pub fn builder() -> crate::input::complete_multipart_upload_input::Builder {
        crate::input::complete_multipart_upload_input::Builder::default()
    }
}

pub mod abort_multipart_upload_input {

    #[derive(
        std::default::Default, std::clone::Clone, std::cmp::PartialEq, Eq, std::fmt::Debug,
    )]
    pub struct Builder {
        pub(crate) bucket: std::option::Option<std::string::String>,
        pub(crate) key: std::option::Option<std::string::String>,
        pub(crate) upload_id: std::option::Option<std::string::String>,
    }
    impl Builder {
        pub fn bucket(mut self, input: impl Into<std::string::String>) -> Self {
            self.bucket = Some(input.into());
            self
        }
        pub fn set_bucket(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.bucket = input;
            self
        }
        pub fn key(mut self, input: impl Into<std::string::String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.key = input;
            self
        }
        pub fn upload_id(mut self, input: impl Into<std::string::String>) -> Self {
            self.upload_id = Some(input.into());
            self
        }
        pub fn set_upload_id(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.upload_id = input;
            self
        }

        pub fn build(
            self,
        ) -> Result<crate::input::AbortMultipartUploadInput, aws_smithy_http::operation::BuildError>
        {
            Ok(crate::input::AbortMultipartUploadInput {
                bucket: self.bucket,
                key: self.key,
                upload_id: self.upload_id,
            })
        }
    }
}

impl AbortMultipartUploadInput {
    pub fn builder() -> crate::input::abort_multipart_upload_input::Builder {
        crate::input::abort_multipart_upload_input::Builder::default()
    }
}

pub mod get_object_input {

    #[derive(
        std::default::Default, std::clone::Clone, std::cmp::PartialEq, Eq, std::fmt::Debug,
    )]
    pub struct Builder {
        pub(crate) bucket: std::option::Option<std::string::String>,
        pub(crate) key: std::option::Option<std::string::String>,
        pub(crate) range: std::option::Option<std::string::String>,
    }
    impl Builder {
        pub fn bucket(mut self, input: impl Into<std::string::String>) -> Self {
            self.bucket = Some(input.into());
            self
        }
        pub fn set_bucket(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.bucket = input;
            self
        }
        pub fn key(mut self, input: impl Into<std::string::String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.key = input;
            self
        }
        pub fn range(mut self, input: impl Into<std::string::String>) -> Self {
            self.range = Some(input.into());
            self
        }
        pub fn set_range(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.range = input;
            self
        }
        pub fn build(
            self,
        ) -> Result<crate::input::GetObjectInput, aws_smithy_http::operation::BuildError> {
            Ok(crate::input::GetObjectInput {
                bucket: self.bucket,
                key: self.key,
                range: self.range,
            })
        }
    }
}

impl GetObjectInput {
    pub fn builder() -> crate::input::get_object_input::Builder {
        crate::input::get_object_input::Builder::default()
    }
}

pub mod put_object_input {

    #[derive(std::default::Default, std::fmt::Debug)]
    pub struct Builder {
        pub(crate) body: std::option::Option<crate::types::ByteStream>,
        pub(crate) bucket: std::option::Option<std::string::String>,
        pub(crate) key: std::option::Option<std::string::String>,
    }
    impl Builder {
        pub fn body(mut self, input: crate::types::ByteStream) -> Self {
            self.body = Some(input);
            self
        }
        pub fn set_body(mut self, input: std::option::Option<crate::types::ByteStream>) -> Self {
            self.body = input;
            self
        }
        pub fn bucket(mut self, input: impl Into<std::string::String>) -> Self {
            self.bucket = Some(input.into());
            self
        }
        pub fn set_bucket(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.bucket = input;
            self
        }
        pub fn key(mut self, input: impl Into<std::string::String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.key = input;
            self
        }
        pub fn build(
            self,
        ) -> Result<crate::input::PutObjectInput, aws_smithy_http::operation::BuildError> {
            Ok(crate::input::PutObjectInput {
                body: self.body.unwrap_or_default(),
                bucket: self.bucket,
                key: self.key,
            })
        }
    }
}

impl PutObjectInput {
    pub fn builder() -> crate::input::put_object_input::Builder {
        crate::input::put_object_input::Builder::default()
    }
}

pub mod delete_object_input {

    #[derive(
        std::default::Default, std::clone::Clone, std::cmp::PartialEq, Eq, std::fmt::Debug,
    )]
    pub struct Builder {
        pub(crate) bucket: std::option::Option<std::string::String>,
        pub(crate) key: std::option::Option<std::string::String>,
    }
    impl Builder {
        pub fn bucket(mut self, input: impl Into<std::string::String>) -> Self {
            self.bucket = Some(input.into());
            self
        }
        pub fn set_bucket(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.bucket = input;
            self
        }
        pub fn key(mut self, input: impl Into<std::string::String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.key = input;
            self
        }

        pub fn build(
            self,
        ) -> Result<crate::input::DeleteObjectInput, aws_smithy_http::operation::BuildError>
        {
            Ok(crate::input::DeleteObjectInput {
                bucket: self.bucket,
                key: self.key,
            })
        }
    }
}

impl DeleteObjectInput {
    pub fn builder() -> crate::input::delete_object_input::Builder {
        crate::input::delete_object_input::Builder::default()
    }
}

pub mod delete_objects_input {

    #[derive(
        std::default::Default, std::clone::Clone, std::cmp::PartialEq, Eq, std::fmt::Debug,
    )]
    pub struct Builder {
        pub(crate) bucket: std::option::Option<std::string::String>,
        pub(crate) delete: std::option::Option<crate::model::Delete>,
    }
    impl Builder {
        pub fn bucket(mut self, input: impl Into<std::string::String>) -> Self {
            self.bucket = Some(input.into());
            self
        }
        pub fn set_bucket(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.bucket = input;
            self
        }
        pub fn delete(mut self, input: crate::model::Delete) -> Self {
            self.delete = Some(input);
            self
        }
        pub fn set_delete(mut self, input: std::option::Option<crate::model::Delete>) -> Self {
            self.delete = input;
            self
        }
        pub fn build(
            self,
        ) -> Result<crate::input::DeleteObjectsInput, aws_smithy_http::operation::BuildError>
        {
            Ok(crate::input::DeleteObjectsInput {
                bucket: self.bucket,
                delete: self.delete,
            })
        }
    }
}
impl DeleteObjectsInput {
    pub fn builder() -> crate::input::delete_objects_input::Builder {
        crate::input::delete_objects_input::Builder::default()
    }
}

pub mod create_multipart_upload_input {

    #[derive(
        std::default::Default, std::clone::Clone, std::cmp::PartialEq, Eq, std::fmt::Debug,
    )]
    pub struct Builder {
        pub(crate) bucket: std::option::Option<std::string::String>,
        pub(crate) key: std::option::Option<std::string::String>,
    }
    impl Builder {
        pub fn bucket(mut self, input: impl Into<std::string::String>) -> Self {
            self.bucket = Some(input.into());
            self
        }
        pub fn set_bucket(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.bucket = input;
            self
        }
        pub fn key(mut self, input: impl Into<std::string::String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.key = input;
            self
        }
        pub fn build(
            self,
        ) -> Result<crate::input::CreateMultipartUploadInput, aws_smithy_http::operation::BuildError>
        {
            Ok(crate::input::CreateMultipartUploadInput {
                bucket: self.bucket,
                key: self.key,
            })
        }
    }
}
impl CreateMultipartUploadInput {
    pub fn builder() -> crate::input::create_multipart_upload_input::Builder {
        crate::input::create_multipart_upload_input::Builder::default()
    }
}

pub mod head_object_input {

    #[derive(
        std::default::Default, std::clone::Clone, std::cmp::PartialEq, Eq, std::fmt::Debug,
    )]
    pub struct Builder {
        pub(crate) bucket: std::option::Option<std::string::String>,
        pub(crate) key: std::option::Option<std::string::String>,
    }
    impl Builder {
        pub fn bucket(mut self, input: impl Into<std::string::String>) -> Self {
            self.bucket = Some(input.into());
            self
        }
        pub fn set_bucket(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.bucket = input;
            self
        }
        pub fn key(mut self, input: impl Into<std::string::String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.key = input;
            self
        }
        pub fn build(
            self,
        ) -> Result<crate::input::HeadObjectInput, aws_smithy_http::operation::BuildError> {
            Ok(crate::input::HeadObjectInput {
                bucket: self.bucket,
                key: self.key,
            })
        }
    }
}
impl HeadObjectInput {
    pub fn builder() -> crate::input::head_object_input::Builder {
        crate::input::head_object_input::Builder::default()
    }
}

pub mod list_objects_v2_input {

    #[derive(
        std::default::Default, std::clone::Clone, std::cmp::PartialEq, Eq, std::fmt::Debug,
    )]
    pub struct Builder {
        pub(crate) bucket: std::option::Option<std::string::String>,
        pub(crate) prefix: std::option::Option<std::string::String>,
        pub(crate) continuation_token: std::option::Option<std::string::String>,
    }
    impl Builder {
        pub fn bucket(mut self, input: impl Into<std::string::String>) -> Self {
            self.bucket = Some(input.into());
            self
        }
        pub fn set_bucket(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.bucket = input;
            self
        }

        pub fn prefix(mut self, input: impl Into<std::string::String>) -> Self {
            self.prefix = Some(input.into());
            self
        }
        pub fn set_prefix(mut self, input: std::option::Option<std::string::String>) -> Self {
            self.prefix = input;
            self
        }
        pub fn continuation_token(mut self, input: impl Into<std::string::String>) -> Self {
            self.continuation_token = Some(input.into());
            self
        }
        pub fn set_continuation_token(
            mut self,
            input: std::option::Option<std::string::String>,
        ) -> Self {
            self.continuation_token = input;
            self
        }

        pub fn build(
            self,
        ) -> Result<crate::input::ListObjectsV2Input, aws_smithy_http::operation::BuildError>
        {
            Ok(crate::input::ListObjectsV2Input {
                bucket: self.bucket,
                prefix: self.prefix,
                continuation_token: self.continuation_token,
            })
        }
    }
}
impl ListObjectsV2Input {
    pub fn builder() -> crate::input::list_objects_v2_input::Builder {
        crate::input::list_objects_v2_input::Builder::default()
    }
}

#[non_exhaustive]
pub struct UploadPartInput {
    pub body: crate::types::ByteStream,
    pub bucket: std::option::Option<std::string::String>,
    pub content_length: i64,
    pub key: std::option::Option<std::string::String>,
    pub part_number: i32,
    pub upload_id: std::option::Option<std::string::String>,
}
impl UploadPartInput {
    pub fn body(&self) -> &crate::types::ByteStream {
        &self.body
    }
    pub fn bucket(&self) -> std::option::Option<&str> {
        self.bucket.as_deref()
    }
    pub fn content_length(&self) -> i64 {
        self.content_length
    }
    pub fn key(&self) -> std::option::Option<&str> {
        self.key.as_deref()
    }
    pub fn part_number(&self) -> i32 {
        self.part_number
    }
    pub fn upload_id(&self) -> std::option::Option<&str> {
        self.upload_id.as_deref()
    }
}
impl std::fmt::Debug for UploadPartInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("UploadPartInput");
        formatter.field("body", &self.body);
        formatter.field("bucket", &self.bucket);
        formatter.field("content_length", &self.content_length);
        formatter.field("key", &self.key);
        formatter.field("part_number", &self.part_number);
        formatter.field("upload_id", &self.upload_id);
        formatter.finish()
    }
}

#[allow(missing_docs)] // documentation missing in model
#[non_exhaustive]
#[derive(std::clone::Clone, std::cmp::PartialEq, Eq)]
pub struct CompleteMultipartUploadInput {
    pub bucket: std::option::Option<std::string::String>,
    pub key: std::option::Option<std::string::String>,
    pub multipart_upload: std::option::Option<crate::model::CompletedMultipartUpload>,
    pub upload_id: std::option::Option<std::string::String>,
}
impl CompleteMultipartUploadInput {
    pub fn bucket(&self) -> std::option::Option<&str> {
        self.bucket.as_deref()
    }
    pub fn key(&self) -> std::option::Option<&str> {
        self.key.as_deref()
    }
    pub fn multipart_upload(&self) -> std::option::Option<&crate::model::CompletedMultipartUpload> {
        self.multipart_upload.as_ref()
    }
    pub fn upload_id(&self) -> std::option::Option<&str> {
        self.upload_id.as_deref()
    }
}
impl std::fmt::Debug for CompleteMultipartUploadInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("CompleteMultipartUploadInput");
        formatter.field("bucket", &self.bucket);
        formatter.field("key", &self.key);
        formatter.field("multipart_upload", &self.multipart_upload);
        formatter.field("upload_id", &self.upload_id);
        formatter.finish()
    }
}

#[non_exhaustive]
#[derive(std::clone::Clone, std::cmp::PartialEq, Eq)]
pub struct AbortMultipartUploadInput {
    pub bucket: std::option::Option<std::string::String>,
    pub key: std::option::Option<std::string::String>,
    pub upload_id: std::option::Option<std::string::String>,
}
impl AbortMultipartUploadInput {
    pub fn bucket(&self) -> std::option::Option<&str> {
        self.bucket.as_deref()
    }
    pub fn key(&self) -> std::option::Option<&str> {
        self.key.as_deref()
    }
    pub fn upload_id(&self) -> std::option::Option<&str> {
        self.upload_id.as_deref()
    }
}
impl std::fmt::Debug for AbortMultipartUploadInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("AbortMultipartUploadInput");
        formatter.field("bucket", &self.bucket);
        formatter.field("key", &self.key);
        formatter.field("upload_id", &self.upload_id);
        formatter.finish()
    }
}

#[non_exhaustive]
#[derive(std::clone::Clone, std::cmp::PartialEq, Eq)]
pub struct GetObjectInput {
    pub bucket: std::option::Option<std::string::String>,
    pub key: std::option::Option<std::string::String>,
    pub range: std::option::Option<std::string::String>,
}
impl GetObjectInput {
    pub fn bucket(&self) -> std::option::Option<&str> {
        self.bucket.as_deref()
    }
    pub fn key(&self) -> std::option::Option<&str> {
        self.key.as_deref()
    }
    pub fn range(&self) -> std::option::Option<&str> {
        self.range.as_deref()
    }
}
impl std::fmt::Debug for GetObjectInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("GetObjectInput");
        formatter.field("bucket", &self.bucket);
        formatter.field("key", &self.key);
        formatter.field("range", &self.range);
        formatter.finish()
    }
}

#[non_exhaustive]
pub struct PutObjectInput {
    pub body: crate::types::ByteStream,
    pub bucket: std::option::Option<std::string::String>,
    pub key: std::option::Option<std::string::String>,
}
impl PutObjectInput {
    pub fn body(&self) -> &crate::types::ByteStream {
        &self.body
    }
    pub fn bucket(&self) -> std::option::Option<&str> {
        self.bucket.as_deref()
    }
    pub fn key(&self) -> std::option::Option<&str> {
        self.key.as_deref()
    }
}
impl std::fmt::Debug for PutObjectInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("PutObjectInput");
        formatter.field("body", &self.body);
        formatter.field("bucket", &self.bucket);
        formatter.field("key", &self.key);
        formatter.finish()
    }
}

#[non_exhaustive]
#[derive(std::clone::Clone, std::cmp::PartialEq, Eq)]
pub struct DeleteObjectInput {
    pub bucket: std::option::Option<std::string::String>,
    pub key: std::option::Option<std::string::String>,
}
impl DeleteObjectInput {
    pub fn bucket(&self) -> std::option::Option<&str> {
        self.bucket.as_deref()
    }
    pub fn key(&self) -> std::option::Option<&str> {
        self.key.as_deref()
    }
}
impl std::fmt::Debug for DeleteObjectInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("DeleteObjectInput");
        formatter.field("bucket", &self.bucket);
        formatter.field("key", &self.key);
        formatter.finish()
    }
}

#[non_exhaustive]
#[derive(std::clone::Clone, std::cmp::PartialEq, Eq)]
pub struct DeleteObjectsInput {
    pub bucket: std::option::Option<std::string::String>,
    pub delete: std::option::Option<crate::model::Delete>,
}
impl DeleteObjectsInput {
    pub fn bucket(&self) -> std::option::Option<&str> {
        self.bucket.as_deref()
    }
    pub fn delete(&self) -> std::option::Option<&crate::model::Delete> {
        self.delete.as_ref()
    }
}
impl std::fmt::Debug for DeleteObjectsInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("DeleteObjectsInput");
        formatter.field("bucket", &self.bucket);
        formatter.field("delete", &self.delete);
        formatter.finish()
    }
}

#[non_exhaustive]
#[derive(std::clone::Clone, std::cmp::PartialEq, Eq)]
pub struct CreateMultipartUploadInput {
    pub bucket: std::option::Option<std::string::String>,
    pub key: std::option::Option<std::string::String>,
}
impl CreateMultipartUploadInput {
    pub fn bucket(&self) -> std::option::Option<&str> {
        self.bucket.as_deref()
    }
    pub fn key(&self) -> std::option::Option<&str> {
        self.key.as_deref()
    }
}
impl std::fmt::Debug for CreateMultipartUploadInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("CreateMultipartUploadInput");
        formatter.field("bucket", &self.bucket);
        formatter.field("key", &self.key);
        formatter.finish()
    }
}

#[non_exhaustive]
#[derive(std::clone::Clone, std::cmp::PartialEq, Eq)]
pub struct HeadObjectInput {
    pub bucket: std::option::Option<std::string::String>,
    pub key: std::option::Option<std::string::String>,
}
impl HeadObjectInput {
    pub fn bucket(&self) -> std::option::Option<&str> {
        self.bucket.as_deref()
    }
    pub fn key(&self) -> std::option::Option<&str> {
        self.key.as_deref()
    }
}
impl std::fmt::Debug for HeadObjectInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("HeadObjectInput");
        formatter.field("bucket", &self.bucket);
        formatter.field("key", &self.key);
        formatter.finish()
    }
}

#[non_exhaustive]
#[derive(std::clone::Clone, std::cmp::PartialEq, Eq)]
pub struct ListObjectsV2Input {
    pub bucket: std::option::Option<std::string::String>,
    pub prefix: std::option::Option<std::string::String>,
    pub continuation_token: std::option::Option<std::string::String>,
}
impl ListObjectsV2Input {
    pub fn bucket(&self) -> std::option::Option<&str> {
        self.bucket.as_deref()
    }
    pub fn prefix(&self) -> std::option::Option<&str> {
        self.prefix.as_deref()
    }
    pub fn continuation_token(&self) -> std::option::Option<&str> {
        self.continuation_token.as_deref()
    }
}
impl std::fmt::Debug for ListObjectsV2Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("ListObjectsV2Input");
        formatter.field("bucket", &self.bucket);
        formatter.field("prefix", &self.prefix);
        formatter.field("continuation_token", &self.continuation_token);
        formatter.finish()
    }
}
