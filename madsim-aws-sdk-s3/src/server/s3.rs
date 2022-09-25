use bytes::Bytes;

use crate::{
    input::{
        AbortMultipartUploadInput, CompleteMultipartUploadInput, CreateMultipartUploadInput,
        DeleteObjectInput, DeleteObjectsInput, GetObjectInput, HeadObjectInput, ListObjectsV2Input,
        PutObjectInput, UploadPartInput,
    },
    output::{
        AbortMultipartUploadOutput, CompleteMultipartUploadOutput, CreateMultipartUploadOutput,
        DeleteObjectOutput, DeleteObjectsOutput, GetObjectOutput, HeadObjectOutput,
        ListObjectsV2Output, PutObjectOutput, UploadPartOutput,
    },
};
use std::{
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap, HashSet,
    },
    sync::{atomic::AtomicUsize, Arc, Mutex},
};

use super::error::{Error::*, Result};

type Bucket = Arc<Mutex<HashMap<String, Object>>>;

struct Object {
    head: ObjectHead,
    bytes: Bytes,
}

impl Object {
    fn new(bytes: Bytes, upload_id: Option<String>) -> Object {
        let mut s = HashSet::new();
        let mut completed = true;
        if let Some(upload_id) = upload_id {
            s.insert(upload_id);
            // if we have a upload_id when creating, the object is not completed now.
            completed = false;
        }
        Self {
            head: ObjectHead {
                last_modified: None,
                content_length: bytes.len() as i64,
                completed,
                upload_id: s,
            },
            bytes,
        }
    }

    fn update(&mut self, bytes: Bytes) {
        // todo: update head
        self.bytes = bytes;
        self.head.completed = true;
    }

    fn head(&self) -> ObjectHead {
        self.head.clone()
    }
}

#[derive(Clone)]
struct ObjectHead {
    last_modified: Option<crate::types::DateTime>,
    content_length: i64,
    completed: bool,
    upload_id: HashSet<String>,
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
    upload_id: AtomicUsize,
}

impl S3 {
    fn new() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            multipart: Arc::new(Mutex::new(HashMap::new())),
            upload_id: AtomicUsize::new(0),
        }
    }

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
            Some(object) if object.head.completed => {
                return Ok(GetObjectOutput {
                    body: object.bytes.clone().into(),
                })
            }
            _ => return Err(InvalidBucket(input.key.unwrap().clone())),
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
                v.insert(Object::new(input.body, None));
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
            Some(object) if object.head.completed => return Ok(object.head().into()),
            _ => return Err(InvalidKey(input.key.clone().unwrap())),
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

        match bucket.entry(input.key.clone().unwrap()) {
            Occupied(mut o) => {
                let obj = o.get_mut();
                if obj.head.completed {
                    if obj.head.upload_id.is_empty() {
                        o.remove();
                    } else {
                        obj.bytes.clear();
                        obj.head.completed = false;
                    }
                    Ok(DeleteObjectOutput {})
                } else {
                    Err(InvalidKey(input.key.unwrap()))
                }
            }
            Vacant(_) => Err(InvalidKey(input.key.unwrap())),
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
                        Some(key) => match bucket.entry(key.clone()) {
                            Occupied(mut o) => {
                                let obj = o.get_mut();
                                if obj.head.completed {
                                    if obj.head.upload_id.is_empty() {
                                        o.remove();
                                    } else {
                                        obj.bytes.clear();
                                        obj.head.completed = false;
                                    }
                                } else {
                                    err.push(crate::model::Error {
                                        key: Some(key),
                                        version_id: None,
                                        code: None,
                                        message: None,
                                    });
                                }
                            }
                            Vacant(_) => {
                                err.push(crate::model::Error {
                                    key: Some(key),
                                    version_id: None,
                                    code: None,
                                    message: None,
                                });
                            }
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
        input: CreateMultipartUploadInput,
    ) -> Result<CreateMultipartUploadOutput> {
        if input.bucket.is_none() {
            return Err(InvalidBucket(
                "Bucket is necessary to create multipart upload".to_string(),
            ));
        } else if input.key.is_none() {
            return Err(InvalidKey(
                "Key is necessary to create multipart upload".to_string(),
            ));
        }

        let bucket = self.get_bucket(input.bucket.as_ref().unwrap())?;
        let mut bucket = bucket.lock().unwrap();

        // todo: maybe we need change the upload_id gen algorithm.
        let upload_id = self
            .upload_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let upload_id = upload_id.to_string();

        match bucket.entry(input.key.clone().unwrap()) {
            Occupied(mut o) => {
                let object = o.get_mut();
                object.head.upload_id.insert(upload_id.clone());
            }
            Vacant(v) => {
                let mut object = Object::new(Bytes::new(), Some(upload_id.clone()));
                v.insert(object);
            }
        }

        let mut multipart = self.multipart.lock().unwrap();
        multipart.insert(upload_id.clone(), vec![]);

        return Ok(CreateMultipartUploadOutput {
            upload_id: Some(upload_id),
        });
    }
    pub fn abort_multipart_upload(
        &self,
        input: AbortMultipartUploadInput,
    ) -> Result<AbortMultipartUploadOutput> {
        if input.bucket.is_none() {
            return Err(InvalidBucket(
                "Bucket is necessary to abort multipart upload".to_string(),
            ));
        } else if input.key.is_none() {
            return Err(InvalidKey(
                "Key is necessary to abort multipart upload".to_string(),
            ));
        } else if input.upload_id.is_none() {
            return Err(InvalidUploadId(
                "upload_id is necessary to abort multipart upload".to_string(),
            ));
        }

        let bucket = self.get_bucket(input.bucket.as_ref().unwrap())?;
        let mut bucket = bucket.lock().unwrap();

        match bucket.entry(input.key.clone().unwrap()) {
            Occupied(mut o) => {
                let obj = o.get_mut();

                if obj.head.upload_id.remove(input.upload_id.as_ref().unwrap()) {
                    if !obj.head.completed && obj.head.upload_id.is_empty() {
                        o.remove();
                    }
                    let mut multipart = self.multipart.lock().unwrap();
                    multipart.remove(&input.upload_id.unwrap());
                    Ok(AbortMultipartUploadOutput {})
                } else {
                    Err(InvalidUploadId(input.upload_id.clone().unwrap()))
                }
            }
            Vacant(_) => Err(InvalidKey(input.key.unwrap())),
        }
    }
    pub fn upload_part(&self, input: UploadPartInput) -> Result<UploadPartOutput> {
        if input.bucket.is_none() {
            return Err(InvalidBucket(
                "Bucket is necessary to upload multipart".to_string(),
            ));
        } else if input.key.is_none() {
            return Err(InvalidKey(
                "Key is necessary to upload multipart".to_string(),
            ));
        } else if input.upload_id.is_none() {
            return Err(InvalidUploadId(
                "upload_id is necessary to upload multipart".to_string(),
            ));
        }

        let bucket = self.get_bucket(input.bucket.as_ref().unwrap())?;
        let mut bucket = bucket.lock().unwrap();

        match bucket.entry(input.key.clone().unwrap()) {
            Occupied(o) => {
                let obj = o.get();
                if obj
                    .head
                    .upload_id
                    .contains(input.upload_id.as_ref().unwrap())
                {
                    let mut multipart = self.multipart.lock().unwrap();
                    match multipart.entry(input.upload_id.clone().unwrap()) {
                        Occupied(mut o) => {
                            o.get_mut().push(input);
                            // todo: we should provide e_tag for error checking
                            Ok(UploadPartOutput { e_tag: None })
                        }
                        Vacant(_) => Err(InvalidBucket(input.upload_id.unwrap())),
                    }
                } else {
                    Err(InvalidUploadId(input.upload_id.unwrap()))
                }
            }
            Vacant(_) => Err(InvalidKey(input.key.unwrap())),
        }
    }
    pub fn complete_multipart_upload(
        &self,
        input: CompleteMultipartUploadInput,
    ) -> Result<CompleteMultipartUploadOutput> {
        if input.bucket.is_none() {
            return Err(InvalidBucket(
                "Bucket is necessary to complete multipart upload".to_string(),
            ));
        } else if input.key.is_none() {
            return Err(InvalidKey(
                "Key is necessary to complete multipart upload".to_string(),
            ));
        } else if input.upload_id.is_none() {
            return Err(InvalidUploadId(
                "upload_id is necessary to complete multipart upload".to_string(),
            ));
        }

        let bucket = self.get_bucket(input.bucket.as_ref().unwrap())?;
        let mut bucket = bucket.lock().unwrap();

        match bucket.entry(input.key.clone().unwrap()) {
            Occupied(mut o) => {
                let object = o.get_mut();
                if object
                    .head
                    .upload_id
                    .contains(input.upload_id.as_ref().unwrap())
                {
                    let mut multipart = self.multipart.lock().unwrap();
                    if let Some(v) = multipart.remove(input.upload_id.as_ref().unwrap()) {
                        let mut completed_part = vec![];
                        if let Some(parts) = input.multipart_upload.as_ref() {
                            if let Some(parts) = parts.parts.as_ref() {
                                for upload_part_input in v.into_iter() {
                                    for part in parts {
                                        // todo: check etag
                                        if upload_part_input.part_number == part.part_number {
                                            completed_part.push(upload_part_input);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        completed_part.sort_by_key(|part| part.part_number);
                        let mut bytes = vec![];
                        for part in &completed_part {
                            bytes.push(part.body.as_ref());
                        }
                        object.update(bytes.concat().into());
                        Ok(CompleteMultipartUploadOutput {})
                    } else {
                        Err(InvalidUploadId(input.upload_id.unwrap()))
                    }
                } else {
                    Err(InvalidUploadId(input.upload_id.unwrap()))
                }
            }
            Vacant(_) => Err(InvalidKey(input.key.unwrap())),
        }
    }

    pub fn list_objects_v2(&self, _input: ListObjectsV2Input) -> Result<ListObjectsV2Output> {
        todo!();
    }
}
