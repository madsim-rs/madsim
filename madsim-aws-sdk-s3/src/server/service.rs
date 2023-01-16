use super::{error::Error, Result};
use crate::input::*;
use crate::model::BucketLifecycleConfiguration;
use crate::model::LifecycleRule;
use crate::output::*;
use madsim::rand::{thread_rng, Rng};
use spin::Mutex;
use std::collections::BTreeMap;
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
        body: crate::types::ByteStream,
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
    }

    pub async fn put_object(
        &self,
        bucket: String,
        key: String,
        object: crate::types::ByteStream,
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
            tracing::warn!(?t, "etcdserver: request timed out");
            return Err(Error::GRpcStatus(tonic::Status::new(
                tonic::Code::Unavailable,
                "etcdserver: request timed out",
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct ServiceInner {
    /// (bucket, key) -> Object
    storage: BTreeMap<String, BTreeMap<String, InnerObject>>,

    /// (bucket) -> LifecycleRules
    lifecycle: BTreeMap<String, Vec<LifecycleRule>>,
}

#[derive(Debug, Default)]
struct InnerObject {
    body: Vec<u8>,

    completed: bool,

    /// upload_id -> parts
    parts: BTreeMap<String, Vec<InnerPart>>,

    last_modified: Option<crate::types::DateTime>,

    content_length: i64,
}

#[derive(Debug, Default)]
struct InnerPart {
    part_number: i32,
    body: Vec<u8>,
    e_tag: String,
}

impl ServiceInner {
    fn create_multipart_upload(
        &mut self,
        bucket: String,
        key: String,
    ) -> Result<CreateMultipartUploadOutput> {
        let object = self
            .storage
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
        body: crate::types::ByteStream,
        _content_length: i64,
        part_number: i32,
        upload_id: String,
    ) -> Result<UploadPartOutput> {
        let object = self
            .storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .get_mut(&key)
            .ok_or(Error::InvalidKey(key))?;

        let parts = object
            .parts
            .get_mut(&upload_id)
            .ok_or(Error::InvalidUploadId(upload_id))?;

        let body = body.collect().await;
        let body = body.expect("error read data").into_bytes().to_vec();

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
        let object = self
            .storage
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
            multipart.sort_by(|part1, part2| part1.part_number.cmp(&part2.part_number));
            let mut selection = vec![];
            for complted_part in multipart {
                for part in parts.iter() {
                    if part.part_number == complted_part.part_number {
                        if let Some(e_tag) = &complted_part.e_tag {
                            if e_tag == &part.e_tag {
                                selection.push(part.body.clone());
                                break;
                            }
                        } else {
                            selection.push(part.body.clone());
                            break;
                        }
                    }
                }
            }

            let body = selection.into_iter().flatten().collect::<Vec<u8>>();
            object.body = body;
            object.completed = true;
            object
                .parts
                .remove(&upload_id)
                .expect("multipart completed, remove upload parts failed");

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
        let object = self
            .storage
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

    fn get_object(
        &self,
        bucket: String,
        key: String,
        range: Option<String>,
        part_number: Option<i32>,
    ) -> Result<GetObjectOutput> {
        let object = self
            .storage
            .get(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .get(&key)
            .ok_or_else(|| Error::InvalidKey(key.clone()))?;

        if !object.completed {
            Err(Error::InvalidKey(key))
        } else if let Some(range) = range {
            // https://www.rfc-editor.org/rfc/rfc9110.html#name-range
            let body = object.body.clone();
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
                let first_pos = range_set
                    .split('-')
                    .next()
                    .ok_or_else(|| Error::InvalidRangeSpecifier(range.clone()))?
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidRangeSpecifier(range.clone()))?;

                // may be just transform the slice, not to_vec()
                body[first_pos..].to_vec()
            } else if range_set.ends_with('-') {
                let end_pos = range_set
                    .split('-')
                    .next()
                    .ok_or_else(|| Error::InvalidRangeSpecifier(range.clone()))?
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidRangeSpecifier(range.clone()))?;

                body[..end_pos].to_vec()
            } else {
                let first_pos = range_set
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

                body[first_pos..end_pos].to_vec()
            };

            let body = crate::types::ByteStream::from(body);
            Ok(GetObjectOutput { body })
        } else if let Some(part_number) = part_number {
            let body = object.body.clone();

            if part_number >= 0 {
                let part_number = part_number as usize;
                if part_number < body.len() {
                    let body =
                        crate::types::ByteStream::from(body[part_number..part_number + 1].to_vec());
                    return Ok(GetObjectOutput { body });
                }
            }
            Err(Error::InvalidPartNumberSpecifier(part_number))
        } else {
            Ok(GetObjectOutput {
                body: crate::types::ByteStream::from(object.body.clone()),
            })
        }
    }

    async fn put_object(
        &mut self,
        bucket: String,
        key: String,
        body: crate::types::ByteStream,
    ) -> Result<PutObjectOutput> {
        let object = self
            .storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .entry(key)
            .or_default();

        let body = body.collect().await;
        let body = body.expect("error read data").into_bytes().to_vec();

        object.body = body;
        object.completed = true;

        Ok(PutObjectOutput {})
    }

    fn delete_object(&mut self, bucket: String, key: String) -> Result<DeleteObjectOutput> {
        let object = self
            .storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?
            .get_mut(&key)
            .ok_or_else(|| Error::InvalidKey(key.clone()))?;

        if !object.completed {
            Err(Error::InvalidKey(key))
        } else {
            object.completed = false;
            object.body.clear();
            Ok(DeleteObjectOutput {})
        }
    }

    fn delete_objects(
        &mut self,
        bucket: String,
        delete: crate::model::Delete,
    ) -> Result<DeleteObjectsOutput> {
        let bucket = self
            .storage
            .get_mut(&bucket)
            .ok_or(Error::InvalidBucket(bucket))?;

        if let Some(delete) = delete.objects {
            let delete = delete
                .into_iter()
                .flat_map(|i| i.key)
                .collect::<Vec<String>>();

            let mut errors = vec![];

            for key in delete {
                if let Some(object) = bucket.get_mut(&key) {
                    object.completed = false;
                    object.body.clear();
                } else {
                    errors.push(crate::model::Error {
                        key: Some(key),
                        version_id: None,
                        code: None,
                        message: None,
                    })
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
        let object = self
            .storage
            .get(&bucket)
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
        let bucket = self
            .storage
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
        use std::collections::btree_map::Entry::{Occupied, Vacant};
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
