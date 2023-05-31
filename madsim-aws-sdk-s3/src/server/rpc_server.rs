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
                use Request::*;

                let response: Payload = match request {
                    CreateMultipartUpload(input) => {
                        Box::new(service.create_multipart_upload(input).await)
                    }
                    UploadPart(input) => Box::new(service.upload_part(input).await),
                    CompletedMultipartUpload(input) => {
                        Box::new(service.complete_multipart_upload(input).await)
                    }
                    AbortMultipartUpload(input) => {
                        Box::new(service.abort_multipart_upload(input).await)
                    }
                    GetObject(input) => Box::new(service.get_object(input).await),
                    PutObject(input) => Box::new(service.put_object(input).await),
                    DeleteObject(input) => Box::new(service.delete_object(input).await),
                    DeleteObjects(input) => Box::new(service.delete_objects(input).await),
                    HeadObject(input) => Box::new(service.head_object(input).await),
                    ListObjectsV2(input) => Box::new(service.list_objects_v2(input).await),
                    PutBucketLifecycleConfiguration(input) => {
                        Box::new(service.put_bucket_lifecycle_configuration(input).await)
                    }
                    GetBucketLifecycleConfiguration(input) => {
                        Box::new(service.get_bucket_lifecycle_configuration(input).await)
                    }
                };
                tx.send(response).await?;
                Ok(()) as Result<()>
            });
        }
    }
}
