use std::{
    convert::Infallible,
    fmt::{Debug, Formatter, Result as FmtResult},
    str::FromStr,
};

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct CompletedMultipartUpload {
    pub parts: Option<Vec<crate::model::CompletedPart>>,
}
impl CompletedMultipartUpload {
    pub fn parts(&self) -> Option<&[crate::model::CompletedPart]> {
        self.parts.as_deref()
    }
}
impl Debug for CompletedMultipartUpload {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("CompletedMultipartUpload");
        formatter.field("parts", &self.parts);
        formatter.finish()
    }
}
pub mod completed_multipart_upload {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) parts: Option<Vec<crate::model::CompletedPart>>,
    }
    impl Builder {
        pub fn parts(mut self, input: crate::model::CompletedPart) -> Self {
            let mut v = self.parts.unwrap_or_default();
            v.push(input);
            self.parts = Some(v);
            self
        }

        pub fn set_parts(mut self, input: Option<Vec<crate::model::CompletedPart>>) -> Self {
            self.parts = input;
            self
        }

        pub fn build(self) -> crate::model::CompletedMultipartUpload {
            crate::model::CompletedMultipartUpload { parts: self.parts }
        }
    }
}
impl CompletedMultipartUpload {
    pub fn builder() -> crate::model::completed_multipart_upload::Builder {
        crate::model::completed_multipart_upload::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct CompletedPart {
    pub e_tag: Option<String>,
    pub part_number: i32,
}
impl CompletedPart {
    pub fn e_tag(&self) -> Option<&str> {
        self.e_tag.as_deref()
    }
    pub fn part_number(&self) -> i32 {
        self.part_number
    }
}
impl Debug for CompletedPart {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("CompletedPart");
        formatter.field("e_tag", &self.e_tag);
        formatter.field("part_number", &self.part_number);
        formatter.finish()
    }
}
pub mod completed_part {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) e_tag: Option<String>,
        pub(crate) part_number: Option<i32>,
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
        pub fn part_number(mut self, input: i32) -> Self {
            self.part_number = Some(input);
            self
        }
        pub fn set_part_number(mut self, input: Option<i32>) -> Self {
            self.part_number = input;
            self
        }
        pub fn build(self) -> crate::model::CompletedPart {
            crate::model::CompletedPart {
                e_tag: self.e_tag,
                part_number: self.part_number.unwrap_or_default(),
            }
        }
    }
}
impl CompletedPart {
    pub fn builder() -> crate::model::completed_part::Builder {
        crate::model::completed_part::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct Delete {
    pub objects: Option<Vec<crate::model::ObjectIdentifier>>,
}
impl Delete {
    pub fn objects(&self) -> Option<&[crate::model::ObjectIdentifier]> {
        self.objects.as_deref()
    }
}
impl Debug for Delete {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("Delete");
        formatter.field("objects", &self.objects);
        formatter.finish()
    }
}
pub mod delete {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) objects: Option<Vec<crate::model::ObjectIdentifier>>,
    }
    impl Builder {
        pub fn objects(mut self, input: crate::model::ObjectIdentifier) -> Self {
            let mut v = self.objects.unwrap_or_default();
            v.push(input);
            self.objects = Some(v);
            self
        }

        pub fn set_objects(mut self, input: Option<Vec<crate::model::ObjectIdentifier>>) -> Self {
            self.objects = input;
            self
        }
        pub fn build(self) -> crate::model::Delete {
            crate::model::Delete {
                objects: self.objects,
            }
        }
    }
}
impl Delete {
    pub fn builder() -> crate::model::delete::Builder {
        crate::model::delete::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct ObjectIdentifier {
    pub key: Option<String>,
}
impl ObjectIdentifier {
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
}
impl Debug for ObjectIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("ObjectIdentifier");
        formatter.field("key", &self.key);
        formatter.finish()
    }
}

pub mod object_identifier {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) key: Option<String>,
    }
    impl Builder {
        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.key = Some(input.into());
            self
        }

        pub fn set_key(mut self, input: Option<String>) -> Self {
            self.key = input;
            self
        }

        pub fn build(self) -> crate::model::ObjectIdentifier {
            crate::model::ObjectIdentifier { key: self.key }
        }
    }
}
impl ObjectIdentifier {
    pub fn builder() -> crate::model::object_identifier::Builder {
        crate::model::object_identifier::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct Error {
    pub key: Option<String>,
    pub version_id: Option<String>,
    pub code: Option<String>,
    pub message: Option<String>,
}
impl Error {
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    pub fn version_id(&self) -> Option<&str> {
        self.version_id.as_deref()
    }
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}
impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("Error");
        formatter.field("key", &self.key);
        formatter.field("version_id", &self.version_id);
        formatter.field("code", &self.code);
        formatter.field("message", &self.message);
        formatter.finish()
    }
}
pub mod error {

    #[derive(Default, Clone, PartialEq, Debug, Eq)]
    pub struct Builder {
        pub(crate) key: Option<String>,
        pub(crate) version_id: Option<String>,
        pub(crate) code: Option<String>,
        pub(crate) message: Option<String>,
    }
    impl Builder {
        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: Option<String>) -> Self {
            self.key = input;
            self
        }
        pub fn version_id(mut self, input: impl Into<String>) -> Self {
            self.version_id = Some(input.into());
            self
        }
        pub fn set_version_id(mut self, input: Option<String>) -> Self {
            self.version_id = input;
            self
        }
        pub fn code(mut self, input: impl Into<String>) -> Self {
            self.code = Some(input.into());
            self
        }

        pub fn set_code(mut self, input: Option<String>) -> Self {
            self.code = input;
            self
        }
        pub fn message(mut self, input: impl Into<String>) -> Self {
            self.message = Some(input.into());
            self
        }
        pub fn set_message(mut self, input: Option<String>) -> Self {
            self.message = input;
            self
        }
        pub fn build(self) -> crate::model::Error {
            crate::model::Error {
                key: self.key,
                version_id: self.version_id,
                code: self.code,
                message: self.message,
            }
        }
    }
}
impl Error {
    pub fn builder() -> crate::model::error::Builder {
        crate::model::error::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
pub struct DeletedObject {
    pub key: Option<String>,
    pub version_id: Option<String>,
    pub delete_marker: bool,
    pub delete_marker_version_id: Option<String>,
}
impl DeletedObject {
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    pub fn version_id(&self) -> Option<&str> {
        self.version_id.as_deref()
    }
    pub fn delete_marker(&self) -> bool {
        self.delete_marker
    }
    pub fn delete_marker_version_id(&self) -> Option<&str> {
        self.delete_marker_version_id.as_deref()
    }
}

pub mod deleted_object {

    #[derive(Clone, PartialEq, Default, Debug)]
    pub struct Builder {
        pub(crate) key: Option<String>,
        pub(crate) version_id: Option<String>,
        pub(crate) delete_marker: Option<bool>,
        pub(crate) delete_marker_version_id: Option<String>,
    }
    impl Builder {
        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: Option<String>) -> Self {
            self.key = input;
            self
        }
        pub fn version_id(mut self, input: impl Into<String>) -> Self {
            self.version_id = Some(input.into());
            self
        }
        pub fn set_version_id(mut self, input: Option<String>) -> Self {
            self.version_id = input;
            self
        }
        pub fn delete_marker(mut self, input: bool) -> Self {
            self.delete_marker = Some(input);
            self
        }
        pub fn set_delete_marker(mut self, input: Option<bool>) -> Self {
            self.delete_marker = input;
            self
        }
        pub fn delete_marker_version_id(mut self, input: impl Into<String>) -> Self {
            self.delete_marker_version_id = Some(input.into());
            self
        }
        pub fn set_delete_marker_version_id(mut self, input: Option<String>) -> Self {
            self.delete_marker_version_id = input;
            self
        }
        pub fn build(self) -> crate::model::DeletedObject {
            crate::model::DeletedObject {
                key: self.key,
                version_id: self.version_id,
                delete_marker: self.delete_marker.unwrap_or_default(),
                delete_marker_version_id: self.delete_marker_version_id,
            }
        }
    }
}
impl DeletedObject {
    pub fn builder() -> crate::model::deleted_object::Builder {
        crate::model::deleted_object::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq)]
pub struct Object {
    pub key: Option<String>,
    pub last_modified: Option<aws_smithy_types::DateTime>,
    pub e_tag: Option<String>,
    pub size: i64,
}
impl Object {
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    pub fn last_modified(&self) -> Option<&aws_smithy_types::DateTime> {
        self.last_modified.as_ref()
    }
    pub fn e_tag(&self) -> Option<&str> {
        self.e_tag.as_deref()
    }
    pub fn size(&self) -> i64 {
        self.size
    }
}
impl Debug for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("Object");
        formatter.field("key", &self.key);
        formatter.field("last_modified", &self.last_modified);
        formatter.field("e_tag", &self.e_tag);
        formatter.field("size", &self.size);
        formatter.finish()
    }
}
pub mod object {

    #[derive(Default, Clone, PartialEq, Debug)]
    pub struct Builder {
        pub(crate) key: Option<String>,
        pub(crate) last_modified: Option<aws_smithy_types::DateTime>,
        pub(crate) e_tag: Option<String>,
        pub(crate) size: Option<i64>,
    }
    impl Builder {
        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: Option<String>) -> Self {
            self.key = input;
            self
        }
        pub fn last_modified(mut self, input: aws_smithy_types::DateTime) -> Self {
            self.last_modified = Some(input);
            self
        }
        pub fn set_last_modified(mut self, input: Option<aws_smithy_types::DateTime>) -> Self {
            self.last_modified = input;
            self
        }

        pub fn e_tag(mut self, input: impl Into<String>) -> Self {
            self.e_tag = Some(input.into());
            self
        }

        pub fn set_e_tag(mut self, input: Option<String>) -> Self {
            self.e_tag = input;
            self
        }
        pub fn size(mut self, input: i64) -> Self {
            self.size = Some(input);
            self
        }
        pub fn set_size(mut self, input: Option<i64>) -> Self {
            self.size = input;
            self
        }
        pub fn build(self) -> crate::model::Object {
            crate::model::Object {
                key: self.key,
                last_modified: self.last_modified,
                e_tag: self.e_tag,
                size: self.size.unwrap_or_default(),
            }
        }
    }
}
impl Object {
    pub fn builder() -> crate::model::object::Builder {
        crate::model::object::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq)]
pub struct LifecycleRule {
    pub expiration: Option<crate::model::LifecycleExpiration>,
    pub id: Option<String>,
    pub prefix: Option<String>,
    pub filter: Option<crate::model::LifecycleRuleFilter>,
    pub status: Option<crate::model::ExpirationStatus>,
    pub transitions: Option<Vec<crate::model::Transition>>,
    pub noncurrent_version_transitions: Option<Vec<crate::model::NoncurrentVersionTransition>>,
    pub noncurrent_version_expiration: Option<crate::model::NoncurrentVersionExpiration>,
    pub abort_incomplete_multipart_upload: Option<crate::model::AbortIncompleteMultipartUpload>,
}
impl LifecycleRule {
    pub fn expiration(&self) -> Option<&crate::model::LifecycleExpiration> {
        self.expiration.as_ref()
    }
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }
    pub fn filter(&self) -> Option<&crate::model::LifecycleRuleFilter> {
        self.filter.as_ref()
    }
    pub fn status(&self) -> Option<&crate::model::ExpirationStatus> {
        self.status.as_ref()
    }
    pub fn transitions(&self) -> Option<&[crate::model::Transition]> {
        self.transitions.as_deref()
    }
    pub fn noncurrent_version_transitions(
        &self,
    ) -> Option<&[crate::model::NoncurrentVersionTransition]> {
        self.noncurrent_version_transitions.as_deref()
    }
    pub fn noncurrent_version_expiration(
        &self,
    ) -> Option<&crate::model::NoncurrentVersionExpiration> {
        self.noncurrent_version_expiration.as_ref()
    }
    pub fn abort_incomplete_multipart_upload(
        &self,
    ) -> Option<&crate::model::AbortIncompleteMultipartUpload> {
        self.abort_incomplete_multipart_upload.as_ref()
    }
}
impl Debug for LifecycleRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("LifecycleRule");
        formatter.field("expiration", &self.expiration);
        formatter.field("id", &self.id);
        formatter.field("prefix", &self.prefix);
        formatter.field("filter", &self.filter);
        formatter.field("status", &self.status);
        formatter.field("transitions", &self.transitions);
        formatter.field(
            "noncurrent_version_transitions",
            &self.noncurrent_version_transitions,
        );
        formatter.field(
            "noncurrent_version_expiration",
            &self.noncurrent_version_expiration,
        );
        formatter.field(
            "abort_incomplete_multipart_upload",
            &self.abort_incomplete_multipart_upload,
        );
        formatter.finish()
    }
}
pub mod lifecycle_rule {

    #[derive(Default, Clone, PartialEq, Debug)]
    pub struct Builder {
        pub(crate) expiration: Option<crate::model::LifecycleExpiration>,
        pub(crate) id: Option<String>,
        pub(crate) prefix: Option<String>,
        pub(crate) filter: Option<crate::model::LifecycleRuleFilter>,
        pub(crate) status: Option<crate::model::ExpirationStatus>,
        pub(crate) transitions: Option<Vec<crate::model::Transition>>,
        pub(crate) noncurrent_version_transitions:
            Option<Vec<crate::model::NoncurrentVersionTransition>>,
        pub(crate) noncurrent_version_expiration: Option<crate::model::NoncurrentVersionExpiration>,
        pub(crate) abort_incomplete_multipart_upload:
            Option<crate::model::AbortIncompleteMultipartUpload>,
    }
    impl Builder {
        pub fn expiration(mut self, input: crate::model::LifecycleExpiration) -> Self {
            self.expiration = Some(input);
            self
        }
        pub fn set_expiration(mut self, input: Option<crate::model::LifecycleExpiration>) -> Self {
            self.expiration = input;
            self
        }
        pub fn id(mut self, input: impl Into<String>) -> Self {
            self.id = Some(input.into());
            self
        }
        pub fn set_id(mut self, input: Option<String>) -> Self {
            self.id = input;
            self
        }
        pub fn prefix(mut self, input: impl Into<String>) -> Self {
            self.prefix = Some(input.into());
            self
        }
        pub fn set_prefix(mut self, input: Option<String>) -> Self {
            self.prefix = input;
            self
        }
        pub fn filter(mut self, input: crate::model::LifecycleRuleFilter) -> Self {
            self.filter = Some(input);
            self
        }
        pub fn set_filter(mut self, input: Option<crate::model::LifecycleRuleFilter>) -> Self {
            self.filter = input;
            self
        }
        pub fn status(mut self, input: crate::model::ExpirationStatus) -> Self {
            self.status = Some(input);
            self
        }
        pub fn set_status(mut self, input: Option<crate::model::ExpirationStatus>) -> Self {
            self.status = input;
            self
        }
        pub fn transitions(mut self, input: crate::model::Transition) -> Self {
            let mut v = self.transitions.unwrap_or_default();
            v.push(input);
            self.transitions = Some(v);
            self
        }
        pub fn set_transitions(mut self, input: Option<Vec<crate::model::Transition>>) -> Self {
            self.transitions = input;
            self
        }
        pub fn noncurrent_version_transitions(
            mut self,
            input: crate::model::NoncurrentVersionTransition,
        ) -> Self {
            let mut v = self.noncurrent_version_transitions.unwrap_or_default();
            v.push(input);
            self.noncurrent_version_transitions = Some(v);
            self
        }
        pub fn set_noncurrent_version_transitions(
            mut self,
            input: Option<Vec<crate::model::NoncurrentVersionTransition>>,
        ) -> Self {
            self.noncurrent_version_transitions = input;
            self
        }
        pub fn noncurrent_version_expiration(
            mut self,
            input: crate::model::NoncurrentVersionExpiration,
        ) -> Self {
            self.noncurrent_version_expiration = Some(input);
            self
        }
        pub fn set_noncurrent_version_expiration(
            mut self,
            input: Option<crate::model::NoncurrentVersionExpiration>,
        ) -> Self {
            self.noncurrent_version_expiration = input;
            self
        }
        pub fn abort_incomplete_multipart_upload(
            mut self,
            input: crate::model::AbortIncompleteMultipartUpload,
        ) -> Self {
            self.abort_incomplete_multipart_upload = Some(input);
            self
        }
        pub fn set_abort_incomplete_multipart_upload(
            mut self,
            input: Option<crate::model::AbortIncompleteMultipartUpload>,
        ) -> Self {
            self.abort_incomplete_multipart_upload = input;
            self
        }
        pub fn build(self) -> crate::model::LifecycleRule {
            crate::model::LifecycleRule {
                expiration: self.expiration,
                id: self.id,
                prefix: self.prefix,
                filter: self.filter,
                status: self.status,
                transitions: self.transitions,
                noncurrent_version_transitions: self.noncurrent_version_transitions,
                noncurrent_version_expiration: self.noncurrent_version_expiration,
                abort_incomplete_multipart_upload: self.abort_incomplete_multipart_upload,
            }
        }
    }
}
impl LifecycleRule {
    pub fn builder() -> crate::model::lifecycle_rule::Builder {
        crate::model::lifecycle_rule::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq)]
pub struct LifecycleExpiration {
    pub date: Option<aws_smithy_types::DateTime>,
    pub days: i32,
    pub expired_object_delete_marker: bool,
}
impl LifecycleExpiration {
    pub fn date(&self) -> Option<&aws_smithy_types::DateTime> {
        self.date.as_ref()
    }
    pub fn days(&self) -> i32 {
        self.days
    }
    pub fn expired_object_delete_marker(&self) -> bool {
        self.expired_object_delete_marker
    }
}
impl Debug for LifecycleExpiration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("LifecycleExpiration");
        formatter.field("date", &self.date);
        formatter.field("days", &self.days);
        formatter.field(
            "expired_object_delete_marker",
            &self.expired_object_delete_marker,
        );
        formatter.finish()
    }
}
pub mod lifecycle_expiration {

    #[derive(Default, Clone, PartialEq, Debug)]
    pub struct Builder {
        pub(crate) date: Option<aws_smithy_types::DateTime>,
        pub(crate) days: Option<i32>,
        pub(crate) expired_object_delete_marker: Option<bool>,
    }
    impl Builder {
        pub fn date(mut self, input: aws_smithy_types::DateTime) -> Self {
            self.date = Some(input);
            self
        }
        pub fn set_date(mut self, input: Option<aws_smithy_types::DateTime>) -> Self {
            self.date = input;
            self
        }
        pub fn days(mut self, input: i32) -> Self {
            self.days = Some(input);
            self
        }
        pub fn set_days(mut self, input: Option<i32>) -> Self {
            self.days = input;
            self
        }
        pub fn expired_object_delete_marker(mut self, input: bool) -> Self {
            self.expired_object_delete_marker = Some(input);
            self
        }
        pub fn set_expired_object_delete_marker(mut self, input: Option<bool>) -> Self {
            self.expired_object_delete_marker = input;
            self
        }
        pub fn build(self) -> crate::model::LifecycleExpiration {
            crate::model::LifecycleExpiration {
                date: self.date,
                days: self.days.unwrap_or_default(),
                expired_object_delete_marker: self.expired_object_delete_marker.unwrap_or_default(),
            }
        }
    }
}
impl LifecycleExpiration {
    pub fn builder() -> crate::model::lifecycle_expiration::Builder {
        crate::model::lifecycle_expiration::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct AbortIncompleteMultipartUpload {
    pub days_after_initiation: i32,
}
impl AbortIncompleteMultipartUpload {
    pub fn days_after_initiation(&self) -> i32 {
        self.days_after_initiation
    }
}
impl Debug for AbortIncompleteMultipartUpload {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("AbortIncompleteMultipartUpload");
        formatter.field("days_after_initiation", &self.days_after_initiation);
        formatter.finish()
    }
}
pub mod abort_incomplete_multipart_upload {

    #[derive(Default, Clone, PartialEq, Eq, Debug)]
    pub struct Builder {
        pub(crate) days_after_initiation: Option<i32>,
    }
    impl Builder {
        pub fn days_after_initiation(mut self, input: i32) -> Self {
            self.days_after_initiation = Some(input);
            self
        }
        pub fn set_days_after_initiation(mut self, input: Option<i32>) -> Self {
            self.days_after_initiation = input;
            self
        }
        pub fn build(self) -> crate::model::AbortIncompleteMultipartUpload {
            crate::model::AbortIncompleteMultipartUpload {
                days_after_initiation: self.days_after_initiation.unwrap_or_default(),
            }
        }
    }
}
impl AbortIncompleteMultipartUpload {
    pub fn builder() -> crate::model::abort_incomplete_multipart_upload::Builder {
        crate::model::abort_incomplete_multipart_upload::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct NoncurrentVersionExpiration {
    pub noncurrent_days: i32,
    pub newer_noncurrent_versions: i32,
}
impl NoncurrentVersionExpiration {
    pub fn noncurrent_days(&self) -> i32 {
        self.noncurrent_days
    }
    pub fn newer_noncurrent_versions(&self) -> i32 {
        self.newer_noncurrent_versions
    }
}
impl Debug for NoncurrentVersionExpiration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("NoncurrentVersionExpiration");
        formatter.field("noncurrent_days", &self.noncurrent_days);
        formatter.field("newer_noncurrent_versions", &self.newer_noncurrent_versions);
        formatter.finish()
    }
}
pub mod noncurrent_version_expiration {

    #[derive(Default, Clone, PartialEq, Eq, Debug)]
    pub struct Builder {
        pub(crate) noncurrent_days: Option<i32>,
        pub(crate) newer_noncurrent_versions: Option<i32>,
    }
    impl Builder {
        pub fn noncurrent_days(mut self, input: i32) -> Self {
            self.noncurrent_days = Some(input);
            self
        }
        pub fn set_noncurrent_days(mut self, input: Option<i32>) -> Self {
            self.noncurrent_days = input;
            self
        }
        pub fn newer_noncurrent_versions(mut self, input: i32) -> Self {
            self.newer_noncurrent_versions = Some(input);
            self
        }
        pub fn set_newer_noncurrent_versions(mut self, input: Option<i32>) -> Self {
            self.newer_noncurrent_versions = input;
            self
        }
        pub fn build(self) -> crate::model::NoncurrentVersionExpiration {
            crate::model::NoncurrentVersionExpiration {
                noncurrent_days: self.noncurrent_days.unwrap_or_default(),
                newer_noncurrent_versions: self.newer_noncurrent_versions.unwrap_or_default(),
            }
        }
    }
}
impl NoncurrentVersionExpiration {
    pub fn builder() -> crate::model::noncurrent_version_expiration::Builder {
        crate::model::noncurrent_version_expiration::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct NoncurrentVersionTransition {
    pub noncurrent_days: i32,
    pub storage_class: Option<crate::model::TransitionStorageClass>,
    pub newer_noncurrent_versions: i32,
}
impl NoncurrentVersionTransition {
    pub fn noncurrent_days(&self) -> i32 {
        self.noncurrent_days
    }
    pub fn storage_class(&self) -> Option<&crate::model::TransitionStorageClass> {
        self.storage_class.as_ref()
    }
    pub fn newer_noncurrent_versions(&self) -> i32 {
        self.newer_noncurrent_versions
    }
}
impl Debug for NoncurrentVersionTransition {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("NoncurrentVersionTransition");
        formatter.field("noncurrent_days", &self.noncurrent_days);
        formatter.field("storage_class", &self.storage_class);
        formatter.field("newer_noncurrent_versions", &self.newer_noncurrent_versions);
        formatter.finish()
    }
}
pub mod noncurrent_version_transition {

    #[derive(Default, Clone, PartialEq, Eq, Debug)]
    pub struct Builder {
        pub(crate) noncurrent_days: Option<i32>,
        pub(crate) storage_class: Option<crate::model::TransitionStorageClass>,
        pub(crate) newer_noncurrent_versions: Option<i32>,
    }
    impl Builder {
        pub fn noncurrent_days(mut self, input: i32) -> Self {
            self.noncurrent_days = Some(input);
            self
        }
        pub fn set_noncurrent_days(mut self, input: Option<i32>) -> Self {
            self.noncurrent_days = input;
            self
        }
        pub fn storage_class(mut self, input: crate::model::TransitionStorageClass) -> Self {
            self.storage_class = Some(input);
            self
        }
        pub fn set_storage_class(
            mut self,
            input: Option<crate::model::TransitionStorageClass>,
        ) -> Self {
            self.storage_class = input;
            self
        }
        pub fn newer_noncurrent_versions(mut self, input: i32) -> Self {
            self.newer_noncurrent_versions = Some(input);
            self
        }
        pub fn set_newer_noncurrent_versions(mut self, input: Option<i32>) -> Self {
            self.newer_noncurrent_versions = input;
            self
        }
        pub fn build(self) -> crate::model::NoncurrentVersionTransition {
            crate::model::NoncurrentVersionTransition {
                noncurrent_days: self.noncurrent_days.unwrap_or_default(),
                storage_class: self.storage_class,
                newer_noncurrent_versions: self.newer_noncurrent_versions.unwrap_or_default(),
            }
        }
    }
}
impl NoncurrentVersionTransition {
    pub fn builder() -> crate::model::noncurrent_version_transition::Builder {
        crate::model::noncurrent_version_transition::Builder::default()
    }
}

#[allow(missing_docs)]
#[non_exhaustive]
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Debug, Hash)]
pub enum TransitionStorageClass {
    #[allow(missing_docs)]
    DeepArchive,
    #[allow(missing_docs)]
    Glacier,
    #[allow(missing_docs)]
    GlacierIr,
    #[allow(missing_docs)]
    IntelligentTiering,
    #[allow(missing_docs)]
    OnezoneIa,
    #[allow(missing_docs)]
    StandardIa,
    Unknown(String),
}
impl From<&str> for TransitionStorageClass {
    fn from(s: &str) -> Self {
        match s {
            "DEEP_ARCHIVE" => TransitionStorageClass::DeepArchive,
            "GLACIER" => TransitionStorageClass::Glacier,
            "GLACIER_IR" => TransitionStorageClass::GlacierIr,
            "INTELLIGENT_TIERING" => TransitionStorageClass::IntelligentTiering,
            "ONEZONE_IA" => TransitionStorageClass::OnezoneIa,
            "STANDARD_IA" => TransitionStorageClass::StandardIa,
            other => TransitionStorageClass::Unknown(other.to_owned()),
        }
    }
}
impl FromStr for TransitionStorageClass {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TransitionStorageClass::from(s))
    }
}
impl TransitionStorageClass {
    pub fn as_str(&self) -> &str {
        match self {
            TransitionStorageClass::DeepArchive => "DEEP_ARCHIVE",
            TransitionStorageClass::Glacier => "GLACIER",
            TransitionStorageClass::GlacierIr => "GLACIER_IR",
            TransitionStorageClass::IntelligentTiering => "INTELLIGENT_TIERING",
            TransitionStorageClass::OnezoneIa => "ONEZONE_IA",
            TransitionStorageClass::StandardIa => "STANDARD_IA",
            TransitionStorageClass::Unknown(s) => s.as_ref(),
        }
    }

    pub fn values() -> &'static [&'static str] {
        &[
            "DEEP_ARCHIVE",
            "GLACIER",
            "GLACIER_IR",
            "INTELLIGENT_TIERING",
            "ONEZONE_IA",
            "STANDARD_IA",
        ]
    }
}
impl AsRef<str> for TransitionStorageClass {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq)]
pub struct Transition {
    pub date: Option<aws_smithy_types::DateTime>,
    pub days: i32,
    pub storage_class: Option<crate::model::TransitionStorageClass>,
}
impl Transition {
    pub fn date(&self) -> Option<&aws_smithy_types::DateTime> {
        self.date.as_ref()
    }
    pub fn days(&self) -> i32 {
        self.days
    }
    pub fn storage_class(&self) -> Option<&crate::model::TransitionStorageClass> {
        self.storage_class.as_ref()
    }
}
impl Debug for Transition {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("Transition");
        formatter.field("date", &self.date);
        formatter.field("days", &self.days);
        formatter.field("storage_class", &self.storage_class);
        formatter.finish()
    }
}
pub mod transition {

    #[derive(Default, Clone, PartialEq, Debug)]
    pub struct Builder {
        pub(crate) date: Option<aws_smithy_types::DateTime>,
        pub(crate) days: Option<i32>,
        pub(crate) storage_class: Option<crate::model::TransitionStorageClass>,
    }
    impl Builder {
        pub fn date(mut self, input: aws_smithy_types::DateTime) -> Self {
            self.date = Some(input);
            self
        }
        pub fn set_date(mut self, input: Option<aws_smithy_types::DateTime>) -> Self {
            self.date = input;
            self
        }
        pub fn days(mut self, input: i32) -> Self {
            self.days = Some(input);
            self
        }
        pub fn set_days(mut self, input: Option<i32>) -> Self {
            self.days = input;
            self
        }
        pub fn storage_class(mut self, input: crate::model::TransitionStorageClass) -> Self {
            self.storage_class = Some(input);
            self
        }
        pub fn set_storage_class(
            mut self,
            input: Option<crate::model::TransitionStorageClass>,
        ) -> Self {
            self.storage_class = input;
            self
        }
        pub fn build(self) -> crate::model::Transition {
            crate::model::Transition {
                date: self.date,
                days: self.days.unwrap_or_default(),
                storage_class: self.storage_class,
            }
        }
    }
}
impl Transition {
    pub fn builder() -> crate::model::transition::Builder {
        crate::model::transition::Builder::default()
    }
}

#[allow(missing_docs)]
#[non_exhaustive]
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Debug, Hash)]
pub enum ExpirationStatus {
    #[allow(missing_docs)]
    Disabled,
    #[allow(missing_docs)]
    Enabled,
    Unknown(String),
}
impl From<&str> for ExpirationStatus {
    fn from(s: &str) -> Self {
        match s {
            "Disabled" => ExpirationStatus::Disabled,
            "Enabled" => ExpirationStatus::Enabled,
            other => ExpirationStatus::Unknown(other.to_owned()),
        }
    }
}
impl FromStr for ExpirationStatus {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ExpirationStatus::from(s))
    }
}
impl ExpirationStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ExpirationStatus::Disabled => "Disabled",
            ExpirationStatus::Enabled => "Enabled",
            ExpirationStatus::Unknown(s) => s.as_ref(),
        }
    }
    pub fn values() -> &'static [&'static str] {
        &["Disabled", "Enabled"]
    }
}
impl AsRef<str> for ExpirationStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LifecycleRuleFilter {
    And(crate::model::LifecycleRuleAndOperator),
    ObjectSizeGreaterThan(i64),
    ObjectSizeLessThan(i64),
    Prefix(String),
    Tag(crate::model::Tag),
    #[non_exhaustive]
    Unknown,
}
impl LifecycleRuleFilter {
    pub fn as_and(&self) -> Result<&crate::model::LifecycleRuleAndOperator, &Self> {
        if let LifecycleRuleFilter::And(val) = &self {
            Ok(val)
        } else {
            Err(self)
        }
    }
    pub fn is_and(&self) -> bool {
        self.as_and().is_ok()
    }
    pub fn as_object_size_greater_than(&self) -> Result<&i64, &Self> {
        if let LifecycleRuleFilter::ObjectSizeGreaterThan(val) = &self {
            Ok(val)
        } else {
            Err(self)
        }
    }
    pub fn is_object_size_greater_than(&self) -> bool {
        self.as_object_size_greater_than().is_ok()
    }
    pub fn as_object_size_less_than(&self) -> Result<&i64, &Self> {
        if let LifecycleRuleFilter::ObjectSizeLessThan(val) = &self {
            Ok(val)
        } else {
            Err(self)
        }
    }
    pub fn is_object_size_less_than(&self) -> bool {
        self.as_object_size_less_than().is_ok()
    }
    pub fn as_prefix(&self) -> Result<&String, &Self> {
        if let LifecycleRuleFilter::Prefix(val) = &self {
            Ok(val)
        } else {
            Err(self)
        }
    }
    pub fn is_prefix(&self) -> bool {
        self.as_prefix().is_ok()
    }
    pub fn as_tag(&self) -> Result<&crate::model::Tag, &Self> {
        if let LifecycleRuleFilter::Tag(val) = &self {
            Ok(val)
        } else {
            Err(self)
        }
    }
    pub fn is_tag(&self) -> bool {
        self.as_tag().is_ok()
    }
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct LifecycleRuleAndOperator {
    pub prefix: Option<String>,
    pub tags: Option<Vec<crate::model::Tag>>,
    pub object_size_greater_than: i64,
    pub object_size_less_than: i64,
}
impl LifecycleRuleAndOperator {
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }
    pub fn tags(&self) -> Option<&[crate::model::Tag]> {
        self.tags.as_deref()
    }
    pub fn object_size_greater_than(&self) -> i64 {
        self.object_size_greater_than
    }
    pub fn object_size_less_than(&self) -> i64 {
        self.object_size_less_than
    }
}
impl Debug for LifecycleRuleAndOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("LifecycleRuleAndOperator");
        formatter.field("prefix", &self.prefix);
        formatter.field("tags", &self.tags);
        formatter.field("object_size_greater_than", &self.object_size_greater_than);
        formatter.field("object_size_less_than", &self.object_size_less_than);
        formatter.finish()
    }
}
pub mod lifecycle_rule_and_operator {

    #[derive(Default, Clone, PartialEq, Eq, Debug)]
    pub struct Builder {
        pub(crate) prefix: Option<String>,
        pub(crate) tags: Option<Vec<crate::model::Tag>>,
        pub(crate) object_size_greater_than: Option<i64>,
        pub(crate) object_size_less_than: Option<i64>,
    }
    impl Builder {
        pub fn prefix(mut self, input: impl Into<String>) -> Self {
            self.prefix = Some(input.into());
            self
        }
        pub fn set_prefix(mut self, input: Option<String>) -> Self {
            self.prefix = input;
            self
        }
        pub fn tags(mut self, input: crate::model::Tag) -> Self {
            let mut v = self.tags.unwrap_or_default();
            v.push(input);
            self.tags = Some(v);
            self
        }
        pub fn set_tags(mut self, input: Option<Vec<crate::model::Tag>>) -> Self {
            self.tags = input;
            self
        }
        pub fn object_size_greater_than(mut self, input: i64) -> Self {
            self.object_size_greater_than = Some(input);
            self
        }
        pub fn set_object_size_greater_than(mut self, input: Option<i64>) -> Self {
            self.object_size_greater_than = input;
            self
        }
        pub fn object_size_less_than(mut self, input: i64) -> Self {
            self.object_size_less_than = Some(input);
            self
        }
        pub fn set_object_size_less_than(mut self, input: Option<i64>) -> Self {
            self.object_size_less_than = input;
            self
        }
        pub fn build(self) -> crate::model::LifecycleRuleAndOperator {
            crate::model::LifecycleRuleAndOperator {
                prefix: self.prefix,
                tags: self.tags,
                object_size_greater_than: self.object_size_greater_than.unwrap_or_default(),
                object_size_less_than: self.object_size_less_than.unwrap_or_default(),
            }
        }
    }
}
impl LifecycleRuleAndOperator {
    pub fn builder() -> crate::model::lifecycle_rule_and_operator::Builder {
        crate::model::lifecycle_rule_and_operator::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct Tag {
    pub key: Option<String>,
    pub value: Option<String>,
}
impl Tag {
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }
}
impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("Tag");
        formatter.field("key", &self.key);
        formatter.field("value", &self.value);
        formatter.finish()
    }
}
pub mod tag {

    #[derive(Default, Clone, PartialEq, Eq, Debug)]
    pub struct Builder {
        pub(crate) key: Option<String>,
        pub(crate) value: Option<String>,
    }
    impl Builder {
        pub fn key(mut self, input: impl Into<String>) -> Self {
            self.key = Some(input.into());
            self
        }
        pub fn set_key(mut self, input: Option<String>) -> Self {
            self.key = input;
            self
        }
        pub fn value(mut self, input: impl Into<String>) -> Self {
            self.value = Some(input.into());
            self
        }
        pub fn set_value(mut self, input: Option<String>) -> Self {
            self.value = input;
            self
        }
        pub fn build(self) -> crate::model::Tag {
            crate::model::Tag {
                key: self.key,
                value: self.value,
            }
        }
    }
}
impl Tag {
    pub fn builder() -> crate::model::tag::Builder {
        crate::model::tag::Builder::default()
    }
}

#[non_exhaustive]
#[derive(Clone, PartialEq)]
pub struct BucketLifecycleConfiguration {
    pub rules: Option<Vec<crate::model::LifecycleRule>>,
}
impl BucketLifecycleConfiguration {
    pub fn rules(&self) -> Option<&[crate::model::LifecycleRule]> {
        self.rules.as_deref()
    }
}
impl Debug for BucketLifecycleConfiguration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut formatter = f.debug_struct("BucketLifecycleConfiguration");
        formatter.field("rules", &self.rules);
        formatter.finish()
    }
}
pub mod bucket_lifecycle_configuration {

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
        pub fn build(self) -> crate::model::BucketLifecycleConfiguration {
            crate::model::BucketLifecycleConfiguration { rules: self.rules }
        }
    }
}
impl BucketLifecycleConfiguration {
    pub fn builder() -> crate::model::bucket_lifecycle_configuration::Builder {
        crate::model::bucket_lifecycle_configuration::Builder::default()
    }
}
