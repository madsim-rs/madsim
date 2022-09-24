use bytes::Bytes;

use crate::{
    model::{CompletedPart, Delete},
    output::{
        AbortMultipartUploadOutput, CreateMultipartUploadOutput, DeleteObjectOutput,
        DeleteObjectsOutput, GetObjectOutput, HeadObjectOutput, ListObjectsV2Output,
        PutObjectOutput, UploadPartOutput,
    },
    types::ByteStream,
};
use std::{
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap,
    },
    sync::{Arc, Mutex},
};

use super::error::{Error::*, Result};

type Bucket = Arc<Mutex<HashMap<String, Object>>>;

struct Object {
    head: ObjectHead,
    bytes: Bytes,
}

impl Object {
    fn new(bytes: Bytes) -> Object {
        Self {
            head: ObjectHead {
                last_modified: None,
                content_length: bytes.len() as i64,
            },
            bytes,
        }
    }

    fn update(&mut self, bytes: Bytes) {
        // todo: update head
        self.bytes = bytes;
    }

    fn head(&self) -> ObjectHead {
        self.head.clone()
    }
}

#[derive(Clone)]
struct ObjectHead {
    last_modified: Option<crate::types::DateTime>,
    content_length: i64,
}

impl From<ObjectHead> for HeadObjectOutput {
    fn from(head: ObjectHead) -> Self {
        HeadObjectOutput {
            last_modified: head.last_modified,
            content_length: head.content_length,
        }
    }
}

struct S3 {
    buckets: Arc<Mutex<HashMap<String, Bucket>>>,
}

impl S3 {
    fn get_bucket(&self, bucket: &String) -> Result<Bucket> {
        let buckets = self.buckets.lock().unwrap();
        match buckets.get(bucket) {
            Some(bucket) => return Ok(bucket.clone()),
            None => return Err(InvalidBucket(bucket.to_string())),
        }
    }

    pub fn get_object(&self, bucket: &String, key: &String) -> Result<GetObjectOutput> {
        let bucket = self.get_bucket(bucket)?;
        let bucket = bucket.lock().unwrap();
        match bucket.get(key) {
            Some(object) => {
                return Ok(GetObjectOutput {
                    body: object.bytes.clone().into(),
                })
            }
            None => return Err(InvalidBucket(key.clone())),
        }
    }

    pub fn put_object(
        &self,
        bucket: &String,
        key: String,
        body: ByteStream,
    ) -> Result<PutObjectOutput> {
        let bucket = self.get_bucket(bucket)?;
        let mut bucket = bucket.lock().unwrap();
        match bucket.entry(key) {
            Occupied(mut o) => {
                let object = o.get_mut();
                object.update(body);
                return Ok(PutObjectOutput {});
            }
            Vacant(v) => {
                v.insert(Object::new(body));
                return Ok(PutObjectOutput {});
            }
        }
    }

    pub fn head_object(&self, bucket: &String, key: &String) -> Result<HeadObjectOutput> {
        let bucket = self.get_bucket(bucket)?;
        let bucket = bucket.lock().unwrap();
        match bucket.get(key) {
            Some(object) => return Ok(object.head().into()),
            None => return Err(InvalidKey(key.clone())),
        }
    }

    pub fn delete_object(&self, bucket: &String, key: &String) -> Result<DeleteObjectOutput> {
        let bucket = self.get_bucket(bucket)?;
        let mut bucket = bucket.lock().unwrap();
        match bucket.remove(key) {
            Some(_) => return Ok(DeleteObjectOutput {}),
            None => return Err(InvalidKey(key.clone())),
        }
    }

    pub fn delete_objects(&self, bucket: &String, delete: Delete) -> Result<DeleteObjectsOutput> {
        let bucket = self.get_bucket(bucket)?;
        let mut bucket = bucket.lock().unwrap();
        match delete.objects {
            Some(objects) => {
                let mut err = vec![];
                for obj in objects {
                    match obj.key {
                        Some(key) => match bucket.remove(&key) {
                            Some(_) => continue,
                            None => err.push(crate::model::Error {
                                key: Some(key),
                                version_id: None,
                                code: None,
                                message: None,
                            }),
                        },
                        None => continue,
                    }
                }
                return Ok(DeleteObjectsOutput { errors: Some(err) });
            }
            None => return Ok(DeleteObjectsOutput { errors: None }),
        }
    }

    pub fn create_multipart_upload(
        &self,
        _bucket: &String,
        _key: &String,
    ) -> Result<CreateMultipartUploadOutput> {
        todo!();
    }
    pub fn abort_multipart_upload(
        &self,
        _bucket: &String,
        _key: &String,
        _upload_id: &String,
    ) -> Result<AbortMultipartUploadOutput> {
        todo!();
    }
    pub fn upload_part(
        &self,
        _bucket: &String,
        _key: &String,
        _upload_id: &String,
        _part_number: i32,
        _content_length: i64,
        _body: ByteStream,
    ) -> Result<UploadPartOutput> {
        todo!();
    }
    pub fn complete_multipart_upload(
        &self,
        _bucket: &String,
        _key: &String,
        _upload_id: &String,
        _completed_part: &CompletedPart,
    ) -> Result<CreateMultipartUploadOutput> {
        todo!();
    }

    pub fn list_objects_v2(
        &self,
        _bucket: &String,
        _prefix: &String,
        _continuation_token: &String,
    ) -> Result<ListObjectsV2Output> {
        todo!();
    }
}
