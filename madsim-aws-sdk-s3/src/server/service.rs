use super::{error::Error, Result};
use crate::input::*;
use crate::model::BucketLifecycleConfiguration;
use crate::model::LifecycleRule;
use crate::output::*;
use crate::sim::types::ByteStream;
use madsim::rand::{thread_rng, Rng};
use spin::Mutex;
use std::collections::btree_map::Entry::*;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::time::Duration;

/// A request to s3 server.
#[derive(Debug)]
pub(crate) enum Request {
    CreateMultipartUpload(CreateMultipartUploadInput),
    UploadPart(UploadPartInput),
    CompletedMultipartUpload(CompleteMultipartUploadInput),
    AbortMultipartUpload(AbortMultipartUploadInput),
    GetObject(GetObjectInput),
    PutObject(PutObjectInput),
    DeleteObject(DeleteObjectInput),
    DeleteObjects(DeleteObjectsInput),
    HeadObject(HeadObjectInput),
    ListObjectsV2(ListObjectsV2Input),
    PutBucketLifecycleConfiguration(PutBucketLifecycleConfigurationInput),
    GetBucketLifecycleConfiguration(GetBucketLifecycleConfigurationInput),
}

#[derive(Debug)]
pub struct S3Service {
    timeout_rate: f32,
    inner: Mutex<ServiceInner>,
}

impl S3Service {
    pub fn new(timeout_rate: f32) -> Self {
        S3Service {
            timeout_rate,
            inner: Mutex::new(ServiceInner::default()),
        }
    }

    pub async fn create_multipart_upload(
        &self,
        bucket: String,
        key: String,
    ) -> Result<CreateMultipartUploadOutput> {
        self.timeout().await?;
        self.inner.lock().create_multipart_upload(bucket, key)
    }

    pub async fn upload_part(
        &self,
        bucket: String,
        key: String,
        body: ByteStream,
        content_length: i64,
        part_number: i32,
        upload_id: String,
    ) -> Result<UploadPartOutput> {
        self.timeout().await?;
        self.inner
            .lock()
            .upload_part(bucket, key, body, content_length, part_number, upload_id)
            .await
    }

    pub async fn complete_multipart_upload(
        &self,
        bucket: String,
        key: String,
        multipart: crate::model::CompletedMultipartUpload,
        upload_id: String,
    ) -> Result<CompleteMultipartUploadOutput> {
        self.timeout().await?;
        self.inner
            .lock()
            .complete_multipart_upload(bucket, key, multipart, upload_id)
    }

    pub async fn abort_multipart_upload(
        &self,
        bucket: String,
        key: String,
        upload_id: String,
    ) -> Result<AbortMultipartUploadOutput> {
        self.timeout().await?;
        self.inner
            .lock()
            .abort_multipart_upload(bucket, key, upload_id)
    }

    pub async fn get_object(
        &self,
        bucket: String,
        key: String,
        range: Option<String>,
        part_number: Option<i32>,
    ) -> Result<GetObjectOutput> {
        self.timeout().await?;
        self.inner
            .lock()
            .get_object(bucket, key, range, part_number)
            .await
    }

    pub async fn put_object(
        &self,
        bucket: String,
        key: String,
        object: ByteStream,
    ) -> Result<PutObjectOutput> {
        self.timeout().await?;
        self.inner.lock().put_object(bucket, key, object).await
    }

    pub async fn delete_object(&self, bucket: String, key: String) -> Result<DeleteObjectOutput> {
        self.timeout().await?;
        self.inner.lock().delete_object(bucket, key)
    }

    pub async fn delete_objects(
        &self,
        bucket: String,
        delete: crate::model::Delete,
    ) -> Result<DeleteObjectsOutput> {
        self.timeout().await?;
        self.inner.lock().delete_objects(bucket, delete)
    }

    pub async fn head_object(&self, bucket: String, key: String) -> Result<HeadObjectOutput> {
        self.timeout().await?;
        self.inner.lock().head_object(bucket, key)
    }

    pub async fn list_objects_v2(
        &self,
        bucket: String,
        prefix: Option<String>,
        continuation_token: Option<String>,
    ) -> Result<ListObjectsV2Output> {
        self.timeout().await?;
        self.inner
            .lock()
            .list_objects_v2(bucket, prefix, continuation_token)
    }

    pub async fn get_bucket_lifecycle_configuration(
        &self,
        bucket: String,
        expected_bucket_owner: Option<String>,
    ) -> Result<GetBucketLifecycleConfigurationOutput> {
        self.timeout().await?;
        self.inner
            .lock()
            .get_bucket_lifecycle_configuration(bucket, expected_bucket_owner)
    }

    pub async fn put_bucket_lifecycle_configuration(
        &self,
        bucket: String,
        lifecycle_configuration: Option<BucketLifecycleConfiguration>,
        expected_bucket_owner: Option<String>,
    ) -> Result<PutBucketLifecycleConfigurationOutput> {
        self.timeout().await?;
        self.inner.lock().put_bucket_lifecycle_configuration(
            bucket,
            lifecycle_configuration.unwrap_or(BucketLifecycleConfiguration {
                rules: Some(Vec::new()),
            }),
            expected_bucket_owner,
        )
    }

    async fn timeout(&self) -> Result<()> {
        if thread_rng().gen_bool(self.timeout_rate as f64) {
            let t = thread_rng().gen_range(Duration::from_secs(5)..Duration::from_secs(15));
            madsim::time::sleep(t).await;
            tracing::warn!(?t, "s3: request timed out");
            return Err(Error::RequestTimeout(tonic::Status::new(
                tonic::Code::Unavailable,
                "s3: request timed out",
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct ServiceInner {
    /// (bucket, key) -> Object
    storage: Mutex<BTreeMap<String, BTreeMap<String, InnerObject>>>,

    /// (bucket) -> LifecycleRules
    lifecycle: BTreeMap<String, Vec<LifecycleRule>>,
}

#[derive(Debug, Default)]
struct InnerObject {
    body: ByteStream,

    completed: bool,

    /// upload_id -> parts
    parts: BTreeMap<String, Vec<InnerPart>>,

    last_modified: Option<crate::types::DateTime>,

    content_length: i64,
}

#[derive(Debug, Default)]
struct InnerPart {
    part_number: i32,
    body: ByteStream,
    e_tag: String,
}

impl ServiceInner {
    fn create_multipart_upload(
        &mut self,
        bucket: String,
        key: String,
    ) -> Result<CreateMultipartUploadOutput> {
        let mut storage = self.storage.lock();
        let object = storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .entry(key)
            .or_default();

        loop {
            let upload_id = thread_rng().gen::<u32>().to_string();
            if object.parts.contains_key(&upload_id) {
                continue;
            } else {
                object.parts.insert(upload_id.clone(), Default::default());
                return Ok(CreateMultipartUploadOutput {
                    upload_id: Some(upload_id),
                });
            }
        }
    }

    async fn upload_part(
        &mut self,
        bucket: String,
        key: String,
        body: ByteStream,
        _content_length: i64,
        part_number: i32,
        upload_id: String,
    ) -> Result<UploadPartOutput> {
        let mut storage = self.storage.lock();
        let object = storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .get_mut(&key)
            .ok_or(Error::InvalidKey(key))?;

        let parts = object
            .parts
            .get_mut(&upload_id)
            .ok_or(Error::InvalidUploadId(upload_id))?;

        let e_tag = thread_rng().gen::<u32>().to_string();
        let part = InnerPart {
            part_number,
            body,
            e_tag: e_tag.clone(),
        };
        parts.push(part);

        let e_tag = Some(e_tag);
        Ok(UploadPartOutput { e_tag })
    }

    fn complete_multipart_upload(
        &mut self,
        bucket: String,
        key: String,
        multipart: crate::model::CompletedMultipartUpload,
        upload_id: String,
    ) -> Result<CompleteMultipartUploadOutput> {
        let mut storage = self.storage.lock();
        let object = storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .get_mut(&key)
            .ok_or(Error::InvalidKey(key))?;

        if !object.parts.contains_key(&upload_id) {
            return Err(Error::InvalidUploadId(upload_id));
        }

        let parts = object
            .parts
            .get_mut(&upload_id)
            .ok_or_else(|| Error::InvalidUploadId(upload_id.clone()))?;

        if let Some(mut multipart) = multipart.parts {
            multipart.sort_by_key(|part| part.part_number);
            let mut selection_idx = vec![];
            for completed_part in multipart {
                for (idx, part) in parts.iter().enumerate() {
                    if part.part_number == completed_part.part_number {
                        if let Some(e_tag) = &completed_part.e_tag {
                            if e_tag == &part.e_tag {
                                selection_idx.push(idx);
                                break;
                            }
                        } else {
                            selection_idx.push(idx);
                            break;
                        }
                    }
                }
            }

            selection_idx.sort();
            let mut selection_idx = VecDeque::from(selection_idx);
            let mut body = vec![];
            let parts = object.parts.remove(&upload_id).unwrap();

            for (idx, part) in parts.into_iter().enumerate() {
                if let Some(next_idx) = selection_idx.front() {
                    if *next_idx != idx {
                        continue;
                    } else {
                        body.push(part.body);
                        selection_idx.pop_front();
                    }
                } else {
                    break;
                }
            }

            object.body = ByteStream::flatten(body);
            object.completed = true;
            object.parts.remove(&upload_id);

            Ok(CompleteMultipartUploadOutput {})
        } else {
            object
                .parts
                .remove(&upload_id)
                .expect("empty complete multipart request, remove upload_id failed");
            Ok(CompleteMultipartUploadOutput {})
        }
    }

    fn abort_multipart_upload(
        &mut self,
        bucket: String,
        key: String,
        upload_id: String,
    ) -> Result<AbortMultipartUploadOutput> {
        let mut storage = self.storage.lock();
        let object = storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .get_mut(&key)
            .ok_or(Error::InvalidKey(key))?;

        object
            .parts
            .remove(&upload_id)
            .ok_or(Error::InvalidUploadId(upload_id))?;
        Ok(AbortMultipartUploadOutput {})
    }

    async fn get_object(
        &self,
        bucket: String,
        key: String,
        range: Option<String>,
        part_number: Option<i32>,
    ) -> Result<GetObjectOutput> {
        let mut storage = self.storage.lock();
        let object = storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .get_mut(&key)
            .ok_or_else(|| Error::InvalidKey(key.clone()))?;

        if !object.completed {
            Err(Error::InvalidKey(key))
        } else if let Some(range) = range {
            // https://www.rfc-editor.org/rfc/rfc9110.html#name-range
            let mut split = range.split('=');
            let range_unit = split
                .next()
                .ok_or_else(|| Error::InvalidRangeSpecifier(range.clone()))?;

            if range_unit != "bytes" {
                return Err(Error::UnsupportRangeUnit(range_unit.to_string()));
            }

            let range_set = split
                .next()
                .ok_or_else(|| Error::InvalidRangeSpecifier(range.clone()))?;

            let body = if range_set.starts_with('-') {
                let begin_pos = range_set
                    .split_once('-')
                    .ok_or_else(|| Error::InvalidRangeSpecifier(range.clone()))?
                    .0
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidRangeSpecifier(range.clone()))?;

                object.body.after(begin_pos).await
            } else if range_set.ends_with('-') {
                let end_pos = range_set
                    .split_once('-')
                    .ok_or_else(|| Error::InvalidRangeSpecifier(range.clone()))?
                    .0
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidRangeSpecifier(range.clone()))?;

                object.body.before(end_pos).await
            } else {
                let begin_pos = range_set
                    .split('-')
                    .next()
                    .ok_or_else(|| Error::InvalidRangeSpecifier(range.clone()))?
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidRangeSpecifier(range.clone()))?;

                let end_pos = range_set
                    .split('-')
                    .next()
                    .ok_or_else(|| Error::InvalidRangeSpecifier(range.clone()))?
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidRangeSpecifier(range.clone()))?;

                object.body.between(begin_pos, end_pos).await
            };

            Ok(GetObjectOutput { body })
        } else if let Some(part_number) = part_number {
            if part_number >= 0 {
                let part_number = part_number as usize;

                if part_number < object.body.len().await {
                    return Ok(GetObjectOutput {
                        body: object.body.between(part_number, part_number + 1).await,
                    });
                }
            }
            Err(Error::InvalidPartNumberSpecifier(part_number))
        } else {
            Ok(GetObjectOutput {
                body: object.body.clone(),
            })
        }
    }

    async fn put_object(
        &mut self,
        bucket: String,
        key: String,
        body: ByteStream,
    ) -> Result<PutObjectOutput> {
        let mut storage = self.storage.lock();
        let object = storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .entry(key)
            .or_default();

        let body = body.collect().await;
        let body = body.expect("error read data").into_bytes().to_vec();

        object.body = ByteStream::from_bytes(body);
        object.completed = true;

        Ok(PutObjectOutput {})
    }

    fn delete_object(&mut self, bucket: String, key: String) -> Result<DeleteObjectOutput> {
        let mut storage = self.storage.lock();
        let object = storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .entry(key.clone());

        match object {
            Vacant(_) => Err(Error::InvalidKey(key)),
            Occupied(mut o) => {
                if !o.get().completed {
                    Err(Error::InvalidKey(key))
                } else if o.get().parts.is_empty() {
                    o.remove();
                    Ok(DeleteObjectOutput {})
                } else {
                    let object = o.get_mut();
                    object.completed = false;
                    object.body.clear();
                    Ok(DeleteObjectOutput {})
                }
            }
        }
    }

    fn delete_objects(
        &mut self,
        bucket: String,
        delete: crate::model::Delete,
    ) -> Result<DeleteObjectsOutput> {
        let mut storage = self.storage.lock();
        let bucket = storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?;

        if let Some(delete) = delete.objects {
            let delete = delete
                .into_iter()
                .flat_map(|i| i.key)
                .collect::<Vec<String>>();

            let mut errors = vec![];

            for key in delete {
                match bucket.entry(key.clone()) {
                    Vacant(_) => errors.push(crate::model::Error {
                        key: Some(key),
                        version_id: None,
                        code: None,
                        message: Some("key not exists".to_string()),
                    }),
                    Occupied(mut o) => {
                        if !o.get().completed {
                            errors.push(crate::model::Error {
                                key: Some(key),
                                version_id: None,
                                code: None,
                                message: Some("key not exists".to_string()),
                            })
                        } else if o.get().parts.is_empty() {
                            o.remove();
                        } else {
                            let object = o.get_mut();
                            object.completed = false;
                            object.body.clear();
                        }
                    }
                }
            }

            Ok(DeleteObjectsOutput {
                errors: Some(errors),
            })
        } else {
            Ok(DeleteObjectsOutput { errors: None })
        }
    }

    fn head_object(&self, bucket: String, key: String) -> Result<HeadObjectOutput> {
        let mut storage = self.storage.lock();
        let object = storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .get(&key)
            .ok_or_else(|| Error::InvalidKey(key.clone()))?;

        if !object.completed {
            Err(Error::InvalidKey(key))
        } else {
            let last_modified = object.last_modified;
            let content_length = object.content_length;
            Ok(HeadObjectOutput {
                last_modified,
                content_length,
            })
        }
    }

    fn list_objects_v2(
        &mut self,
        bucket: String,
        prefix: Option<String>,
        _continuation_token: Option<String>,
    ) -> Result<ListObjectsV2Output> {
        let mut storage = self.storage.lock();
        let bucket = storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?;

        if let Some(prefix) = prefix {
            let objects = bucket
                .iter()
                .filter(|(key, object)| key.starts_with(&prefix) && object.completed)
                .map(|(key, object)| crate::model::Object {
                    key: Some(key.clone()),
                    last_modified: None,
                    e_tag: None,
                    size: object.content_length,
                })
                .collect();
            Ok(ListObjectsV2Output {
                is_truncated: false,
                contents: Some(objects),
                next_continuation_token: None,
            })
        } else {
            Ok(ListObjectsV2Output {
                is_truncated: false,
                contents: Some(
                    bucket
                        .iter()
                        .map(|(key, object)| crate::model::Object {
                            key: Some(key.clone()),
                            last_modified: None,
                            e_tag: None,
                            size: object.content_length,
                        })
                        .collect(),
                ),
                next_continuation_token: None,
            })
        }
    }

    fn get_bucket_lifecycle_configuration(
        &mut self,
        bucket: String,
        _expected_bucket_owner: Option<String>,
    ) -> Result<GetBucketLifecycleConfigurationOutput> {
        let lifecycle = match self.lifecycle.entry(bucket) {
            Vacant(v) => {
                v.insert(Vec::new());
                Vec::new()
            }
            Occupied(o) => o.get().clone(),
        };

        Ok(GetBucketLifecycleConfigurationOutput {
            rules: Some(lifecycle),
        })
    }

    fn put_bucket_lifecycle_configuration(
        &mut self,
        bucket: String,
        lifecycle_configuration: BucketLifecycleConfiguration,
        _expected_bucket_owner: Option<String>,
    ) -> Result<PutBucketLifecycleConfigurationOutput> {
        self.lifecycle
            .insert(bucket, lifecycle_configuration.rules.unwrap_or_default());

        Ok(PutBucketLifecycleConfigurationOutput {})
    }
}
