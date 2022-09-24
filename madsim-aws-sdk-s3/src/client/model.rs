use std::fmt::{Debug, Formatter};

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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

        pub fn set_e_tag(mut self, input: std::option::Option<String>) -> Self {
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
