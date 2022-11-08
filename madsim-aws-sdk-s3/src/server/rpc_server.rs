use madsim::net::{Endpoint, Payload};
use std::{io::Result, net::SocketAddr, sync::Arc};

use super::{service::Request, service::S3Service};

/// A simulated s3 server.
#[derive(Default, Clone)]
pub struct SimServer {
    timeout_rate: f32,
}

impl SimServer {
    pub fn builder() -> Self {
        SimServer::default()
    }

    pub fn timeout_rate(mut self, rate: f32) -> Self {
        assert!((0.0..=1.0).contains(&rate));
        self.timeout_rate = rate;
        self
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<()> {
        let ep = Endpoint::bind(addr).await?;
        let service = Arc::new(S3Service::new(self.timeout_rate));
        loop {
            let (tx, mut rx, _) = ep.accept1().await?;
            let service = service.clone();
            madsim::task::spawn(async move {
                let request = *rx.recv().await?.downcast::<Request>().unwrap();

                use super::error::Error;
                use crate::input::*;
                use Request::*;

                let response: Payload = match request {
                    CreateMultipartUpload(CreateMultipartUploadInput { bucket, key }) => {
                        Box::new(if let Some(bucket) = bucket {
                            if let Some(key) = key {
                                service.create_multipart_upload(bucket, key).await
                            } else {
                                Err(Error::InvalidKey("no key".to_string()))
                            }
                        } else {
                            Err(Error::InvalidBucket("no bucket".to_string()))
                        })
                    }
                    UploadPart(UploadPartInput {
                        body,
                        bucket,
                        content_length,
                        key,
                        part_number,
                        upload_id,
                    }) => Box::new(if let Some(bucket) = bucket {
                        if let Some(key) = key {
                            if let Some(upload_id) = upload_id {
                                service
                                    .upload_part(
                                        bucket,
                                        key,
                                        body,
                                        content_length,
                                        part_number,
                                        upload_id,
                                    )
                                    .await
                            } else {
                                Err(Error::InvalidUploadId("no upload_id".to_string()))
                            }
                        } else {
                            Err(Error::InvalidKey("no key".to_string()))
                        }
                    } else {
                        Err(Error::InvalidBucket("no bucket".to_string()))
                    }),
                    CompletedMultipartUpload(CompleteMultipartUploadInput {
                        bucket,
                        key,
                        multipart_upload,
                        upload_id,
                    }) => Box::new(if let Some(bucket) = bucket {
                        if let Some(key) = key {
                            if let Some(upload_id) = upload_id {
                                let multipart = match multipart_upload {
                                    Some(multipart) => multipart,
                                    None => crate::model::CompletedMultipartUpload { parts: None },
                                };
                                service
                                    .complete_multipart_upload(bucket, key, multipart, upload_id)
                                    .await
                            } else {
                                Err(Error::InvalidUploadId("no upload_id".to_string()))
                            }
                        } else {
                            Err(Error::InvalidKey("no key".to_string()))
                        }
                    } else {
                        Err(Error::InvalidBucket("no bucket".to_string()))
                    }),
                    AbortMultipartUpload(AbortMultipartUploadInput {
                        bucket,
                        key,
                        upload_id,
                    }) => Box::new(if let Some(bucket) = bucket {
                        if let Some(key) = key {
                            if let Some(upload_id) = upload_id {
                                service.abort_multipart_upload(bucket, key, upload_id).await
                            } else {
                                Err(Error::InvalidUploadId("no upload_id".to_string()))
                            }
                        } else {
                            Err(Error::InvalidKey("no key".to_string()))
                        }
                    } else {
                        Err(Error::InvalidBucket("no bucket".to_string()))
                    }),
                    GetObject(GetObjectInput { bucket, key, range }) => {
                        Box::new(if let Some(bucket) = bucket {
                            if let Some(key) = key {
                                service.get_object(bucket, key, range).await
                            } else {
                                Err(Error::InvalidKey("no key".to_string()))
                            }
                        } else {
                            Err(Error::InvalidBucket("no bucket".to_string()))
                        })
                    }
                    PutObject(PutObjectInput { body, bucket, key }) => {
                        Box::new(if let Some(bucket) = bucket {
                            if let Some(key) = key {
                                service.put_object(bucket, key, body).await
                            } else {
                                Err(Error::InvalidKey("no key".to_string()))
                            }
                        } else {
                            Err(Error::InvalidBucket("no bucket".to_string()))
                        })
                    }
                    DeleteObject(DeleteObjectInput { bucket, key }) => {
                        Box::new(if let Some(bucket) = bucket {
                            if let Some(key) = key {
                                service.delete_object(bucket, key).await
                            } else {
                                Err(Error::InvalidKey("no key".to_string()))
                            }
                        } else {
                            Err(Error::InvalidBucket("no bucket".to_string()))
                        })
                    }
                    DeleteObjects(DeleteObjectsInput { bucket, delete }) => {
                        Box::new(if let Some(bucket) = bucket {
                            let delete = match delete {
                                Some(delete) => delete,
                                None => crate::model::Delete { objects: None },
                            };
                            service.delete_objects(bucket, delete).await
                        } else {
                            Err(Error::InvalidBucket("no bucket".to_string()))
                        })
                    }
                    HeadObject(HeadObjectInput { bucket, key }) => {
                        Box::new(if let Some(bucket) = bucket {
                            if let Some(key) = key {
                                service.head_object(bucket, key).await
                            } else {
                                Err(Error::InvalidKey("no key".to_string()))
                            }
                        } else {
                            Err(Error::InvalidBucket("no bucket".to_string()))
                        })
                    }
                    ListObjectsV2(ListObjectsV2Input {
                        bucket,
                        prefix,
                        continuation_token,
                    }) => Box::new(if let Some(bucket) = bucket {
                        service
                            .list_objects_v2(bucket, prefix, continuation_token)
                            .await
                    } else {
                        Err(Error::InvalidBucket("no bucket".to_string()))
                    }),
                };
                tx.send(response).await?;
                Ok(()) as Result<()>
            });
        }
    }
}
