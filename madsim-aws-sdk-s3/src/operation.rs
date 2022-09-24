#[derive(std::default::Default, std::clone::Clone, std::fmt::Debug)]
pub struct UploadPart {
    _private: (),
}
impl UploadPart {
    pub fn builder() -> crate::input::upload_part_input::Builder {
        crate::input::upload_part_input::Builder::default()
    }
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(std::default::Default, std::clone::Clone, std::fmt::Debug)]
pub struct CompleteMultipartUpload {
    _private: (),
}
impl CompleteMultipartUpload {
    pub fn builder() -> crate::input::complete_multipart_upload_input::Builder {
        crate::input::complete_multipart_upload_input::Builder::default()
    }
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(std::default::Default, std::clone::Clone, std::fmt::Debug)]
pub struct AbortMultipartUpload {
    _private: (),
}
impl AbortMultipartUpload {
    pub fn builder() -> crate::input::abort_multipart_upload_input::Builder {
        crate::input::abort_multipart_upload_input::Builder::default()
    }
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(std::default::Default, std::clone::Clone, std::fmt::Debug)]
pub struct GetObject {
    _private: (),
}
impl GetObject {
    pub fn builder() -> crate::input::get_object_input::Builder {
        crate::input::get_object_input::Builder::default()
    }
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(std::default::Default, std::clone::Clone, std::fmt::Debug)]
pub struct PutObject {
    _private: (),
}
impl PutObject {
    pub fn builder() -> crate::input::put_object_input::Builder {
        crate::input::put_object_input::Builder::default()
    }
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(std::default::Default, std::clone::Clone, std::fmt::Debug)]
pub struct DeleteObject {
    _private: (),
}
impl DeleteObject {
    pub fn builder() -> crate::input::delete_object_input::Builder {
        crate::input::delete_object_input::Builder::default()
    }
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(std::default::Default, std::clone::Clone, std::fmt::Debug)]
pub struct DeleteObjects {
    _private: (),
}
impl DeleteObjects {
    pub fn builder() -> crate::input::delete_objects_input::Builder {
        crate::input::delete_objects_input::Builder::default()
    }
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(std::default::Default, std::clone::Clone, std::fmt::Debug)]
pub struct CreateMultipartUpload {
    _private: (),
}
impl CreateMultipartUpload {
    pub fn builder() -> crate::input::create_multipart_upload_input::Builder {
        crate::input::create_multipart_upload_input::Builder::default()
    }
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(std::default::Default, std::clone::Clone, std::fmt::Debug)]
pub struct HeadObject {
    _private: (),
}
impl HeadObject {
    pub fn builder() -> crate::input::head_object_input::Builder {
        crate::input::head_object_input::Builder::default()
    }
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(std::default::Default, std::clone::Clone, std::fmt::Debug)]
pub struct ListObjectsV2 {
    _private: (),
}
impl ListObjectsV2 {
    pub fn builder() -> crate::input::list_objects_v2_input::Builder {
        crate::input::list_objects_v2_input::Builder::default()
    }
    pub fn new() -> Self {
        Self { _private: () }
    }
}
