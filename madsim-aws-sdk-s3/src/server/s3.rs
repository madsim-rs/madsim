use bytes::Bytes;

use crate::{
    input::{
        AbortMultipartUploadInput, CompleteMultipartUploadInput, CreateMultipartUploadInput,
        DeleteObjectInput, DeleteObjectsInput, GetObjectInput, HeadObjectInput, ListObjectsV2Input,
        PutObjectInput, UploadPartInput,
    },
    output::{
        AbortMultipartUploadOutput, CreateMultipartUploadOutput, DeleteObjectOutput,
        DeleteObjectsOutput, GetObjectOutput, HeadObjectOutput, ListObjectsV2Output,
        PutObjectOutput, UploadPartOutput,
    },
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
    multipart: Arc<Mutex<HashMap<String, Vec<UploadPartInput>>>>,
}

impl S3 {
    fn get_bucket(&self, bucket: &String) -> Result<Bucket> {
        let buckets = self.buckets.lock().unwrap();
        match buckets.get(bucket) {
            Some(bucket) => return Ok(bucket.clone()),
            None => return Err(InvalidBucket(bucket.to_string())),
        }
    }

    pub fn get_object(&self, input: GetObjectInput) -> Result<GetObjectOutput> {
        if input.bucket.is_none() {
            return Err(InvalidBucket(
                "Bucket is necessary to get object".to_string(),
            ));
        } else if input.key.is_none() {
            return Err(InvalidKey("Key is necessary to get object".to_string()));
        }
        let bucket = self.get_bucket(input.bucket.as_ref().unwrap())?;
        let bucket = bucket.lock().unwrap();
        match bucket.get(input.key.as_ref().unwrap()) {
            Some(object) => {
                return Ok(GetObjectOutput {
                    body: object.bytes.clone().into(),
                })
            }
            None => return Err(InvalidBucket(input.key.unwrap().clone())),
        }
    }

    pub fn put_object(&self, input: PutObjectInput) -> Result<PutObjectOutput> {
        if input.bucket.is_none() {
            return Err(InvalidBucket(
                "Bucket is necessary to put object".to_string(),
            ));
        } else if input.key.is_none() {
            return Err(InvalidKey("Key is necessary to put object".to_string()));
        }
        let bucket = self.get_bucket(input.bucket.as_ref().unwrap())?;
        let mut bucket = bucket.lock().unwrap();
        match bucket.entry(input.key.clone().unwrap()) {
            Occupied(mut o) => {
                let object = o.get_mut();
                object.update(input.body);
                return Ok(PutObjectOutput {});
            }
            Vacant(v) => {
                v.insert(Object::new(input.body));
                return Ok(PutObjectOutput {});
            }
        }
    }

    pub fn head_object(&self, input: HeadObjectInput) -> Result<HeadObjectOutput> {
        if input.bucket.is_none() {
            return Err(InvalidBucket(
                "Bucket is necessary to get object head".to_string(),
            ));
        } else if input.key.is_none() {
            return Err(InvalidKey(
                "Key is necessary to get object head".to_string(),
            ));
        }
        let bucket = self.get_bucket(input.bucket.as_ref().unwrap())?;
        let bucket = bucket.lock().unwrap();
        match bucket.get(input.key.as_ref().unwrap()) {
            Some(object) => return Ok(object.head().into()),
            None => return Err(InvalidKey(input.key.clone().unwrap())),
        }
    }

    pub fn delete_object(&self, input: DeleteObjectInput) -> Result<DeleteObjectOutput> {
        if input.bucket.is_none() {
            return Err(InvalidBucket(
                "Bucket is necessary to delete object".to_string(),
            ));
        } else if input.key.is_none() {
            return Err(InvalidKey("Key is necessary to delete object".to_string()));
        }
        let bucket = self.get_bucket(input.bucket.as_ref().unwrap())?;
        let mut bucket = bucket.lock().unwrap();
        match bucket.remove(input.key.as_ref().unwrap()) {
            Some(_) => return Ok(DeleteObjectOutput {}),
            None => return Err(InvalidKey(input.key.clone().unwrap())),
        }
    }

    pub fn delete_objects(&self, input: DeleteObjectsInput) -> Result<DeleteObjectsOutput> {
        if input.bucket.is_none() {
            return Err(InvalidBucket(
                "Bucket is necessary to delete objects".to_string(),
            ));
        }
        if input.delete.is_none() {
            return Ok(DeleteObjectsOutput { errors: None });
        }
        let bucket = self.get_bucket(input.bucket.as_ref().unwrap())?;
        let mut bucket = bucket.lock().unwrap();
        match input.delete.unwrap().objects {
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
        _input: CreateMultipartUploadInput,
    ) -> Result<CreateMultipartUploadOutput> {
        todo!();
    }
    pub fn abort_multipart_upload(
        &self,
        _input: AbortMultipartUploadInput,
    ) -> Result<AbortMultipartUploadOutput> {
        todo!();
    }
    pub fn upload_part(&self, _input: UploadPartInput) -> Result<UploadPartOutput> {
        todo!();
    }
    pub fn complete_multipart_upload(
        &self,
        _input: CompleteMultipartUploadInput,
    ) -> Result<CreateMultipartUploadOutput> {
        todo!();
    }

    pub fn list_objects_v2(&self, _input: ListObjectsV2Input) -> Result<ListObjectsV2Output> {
        todo!();
    }
}
