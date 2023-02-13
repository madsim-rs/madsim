use madsim::net::{Endpoint, Payload};
use std::{io::Result, net::SocketAddr, sync::Arc};

use super::{service::Request, service::S3Service};

/// A simulated s3 server.
#[derive(Default, Clone)]
pub struct SimServer {
    bucket: Option<String>,
}

impl SimServer {
    pub fn builder() -> Self {
        SimServer::default()
    }

    pub fn with_bucket(mut self, bucket: &str) -> Self {
        self.bucket = Some(bucket.into());
        self
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<()> {
        let ep = Endpoint::bind(addr).await?;
        let mut service = S3Service::new();
        if let Some(bucket) = self.bucket {
            service.create_bucket(&bucket).await;
        }
        let service = Arc::new(service);
        loop {
            let (tx, mut rx, _) = ep.accept1().await?;
            let service = service.clone();
            madsim::task::spawn(async move {
                let request = *rx.recv().await?.downcast::<Request>().unwrap();

                use crate::input::*;
                use Request::*;

                let response: Payload = match request {
                    CreateMultipartUpload(CreateMultipartUploadInput { bucket, key }) => {
                        Box::new(service.create_multipart_upload(bucket, key).await)
                    }
                    UploadPart(UploadPartInput {
                        body0,
                        bucket,
                        content_length,
                        key,
                        part_number,
                        upload_id,
                        ..
                    }) => Box::new(
                        service
                            .upload_part(bucket, key, body0, content_length, part_number, upload_id)
                            .await,
                    ),
                    CompletedMultipartUpload(CompleteMultipartUploadInput {
                        bucket,
                        key,
                        multipart_upload,
                        upload_id,
                    }) => Box::new(
                        service
                            .complete_multipart_upload(bucket, key, multipart_upload, upload_id)
                            .await,
                    ),
                    AbortMultipartUpload(AbortMultipartUploadInput {
                        bucket,
                        key,
                        upload_id,
                    }) => Box::new(service.abort_multipart_upload(bucket, key, upload_id).await),
                    GetObject(GetObjectInput {
                        bucket,
                        key,
                        range,
                        part_number,
                    }) => Box::new(service.get_object(bucket, key, range, part_number).await),
                    PutObject(PutObjectInput {
                        body0, bucket, key, ..
                    }) => Box::new(service.put_object(bucket, key, body0).await),
                    DeleteObject(DeleteObjectInput { bucket, key }) => {
                        Box::new(service.delete_object(bucket, key).await)
                    }
                    DeleteObjects(DeleteObjectsInput { bucket, delete }) => {
                        Box::new(service.delete_objects(bucket, delete).await)
                    }
                    HeadObject(HeadObjectInput { bucket, key }) => {
                        Box::new(service.head_object(bucket, key).await)
                    }
                    ListObjectsV2(ListObjectsV2Input {
                        bucket,
                        prefix,
                        continuation_token,
                    }) => Box::new(
                        service
                            .list_objects_v2(bucket, prefix, continuation_token)
                            .await,
                    ),
                    PutBucketLifecycleConfiguration(PutBucketLifecycleConfigurationInput {
                        bucket,
                        lifecycle_configuration,
                        expected_bucket_owner,
                    }) => Box::new(
                        service
                            .put_bucket_lifecycle_configuration(
                                bucket,
                                lifecycle_configuration,
                                expected_bucket_owner,
                            )
                            .await,
                    ),
                    GetBucketLifecycleConfiguration(GetBucketLifecycleConfigurationInput {
                        bucket,
                        expected_bucket_owner,
                    }) => Box::new(
                        service
                            .get_bucket_lifecycle_configuration(bucket, expected_bucket_owner)
                            .await,
                    ),
                };
                tx.send(response).await?;
                Ok(()) as Result<()>
            });
        }
    }
}
