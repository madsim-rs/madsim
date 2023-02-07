pub mod client;
pub use client::*;
pub mod server;

pub use aws_smithy_http::endpoint::Endpoint;
pub use aws_types::region::Region;
pub use aws_types::Credentials;

pub mod types {
    use std::sync::Arc;

    pub use crate::client::result::SdkError;
    use aws_smithy_http::body::SdkBody;
    use aws_smithy_http::byte_stream::AggregatedBytes as AwsAggregatedBytes;
    use aws_smithy_http::byte_stream::ByteStream as AwsByteStream;
    use aws_smithy_http::byte_stream::Error as AwsByteStreamError;
    pub use aws_smithy_types::Blob;
    pub use aws_smithy_types::DateTime;
    use bytes::Buf;
    use bytes::Bytes;
    use bytes_utils::SegmentedBuf;
    use spin::mutex::Mutex;

    #[derive(Debug, Clone)]
    pub enum InnerAggregatedBytes {
        Aws(AwsAggregatedBytes),
        Madsim(SegmentedBuf<Bytes>),
    }

    pub struct AggregatedBytes {
        inner: Vec<InnerAggregatedBytes>,
    }

    impl InnerAggregatedBytes {
        pub fn into_bytes(mut self) -> Bytes {
            match self {
                InnerAggregatedBytes::Aws(b) => b.into_bytes(),
                InnerAggregatedBytes::Madsim(mut b) => b.copy_to_bytes(b.remaining()),
            }
        }
    }

    impl AggregatedBytes {
        pub fn into_bytes(mut self) -> Bytes {
            let mut v = vec![];
            for iag in self.inner {
                let bytes = Vec::<u8>::from(iag.into_bytes());
                v.extend(bytes);
            }
            Bytes::from(v)
        }
    }

    impl Buf for InnerAggregatedBytes {
        fn remaining(&self) -> usize {
            match &self {
                InnerAggregatedBytes::Aws(b) => b.remaining(),
                InnerAggregatedBytes::Madsim(b) => b.remaining(),
            }
        }

        fn chunk(&self) -> &[u8] {
            todo!();
        }

        fn advance(&mut self, _cnt: usize) {
            todo!()
        }

        fn copy_to_bytes(&mut self, len: usize) -> Bytes {
            match self {
                InnerAggregatedBytes::Aws(b) => b.copy_to_bytes(len),
                InnerAggregatedBytes::Madsim(b) => b.copy_to_bytes(len),
            }
        }
    }

    impl Buf for AggregatedBytes {
        fn remaining(&self) -> usize {
            let mut rem = 0;
            for iag in self.inner.iter() {
                rem += iag.remaining();
            }
            rem
        }

        fn chunk(&self) -> &[u8] {
            todo!()
        }

        fn advance(&mut self, _cnt: usize) {
            todo!()
        }

        fn copy_to_bytes(&mut self, mut len: usize) -> Bytes {
            let mut v = vec![];
            for iag in self.inner.iter_mut() {
                if len == 0 {
                    break;
                }
                let bytes = Vec::<u8>::from(iag.copy_to_bytes(len));
                let l = bytes.len();
                if l <= len {
                    v.extend(bytes);
                    len -= l;
                } else {
                    v.extend(bytes[..len].iter())
                }
            }
            Bytes::from(v)
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct ByteStream {
        inner: Vec<Arc<Mutex<Option<InnerByteStream>>>>,
    }

    #[derive(Debug)]
    pub(crate) enum InnerByteStream {
        ByteStream(AwsByteStream),
        Bytes(Vec<u8>),
    }

    impl ByteStream {
        pub fn new(body: SdkBody) -> Self {
            Self {
                inner: vec![Arc::new(Mutex::new(Some(InnerByteStream::ByteStream(
                    AwsByteStream::new(body),
                ))))],
            }
        }

        pub fn clear(&mut self) {
            self.inner.clear();
        }

        pub fn from_static(bytes: &'static [u8]) -> Self {
            Self {
                inner: vec![Arc::new(Mutex::new(Some(InnerByteStream::ByteStream(
                    AwsByteStream::from_static(bytes),
                ))))],
            }
        }

        pub fn from_bytes(bytes: Vec<u8>) -> Self {
            Self {
                inner: vec![Arc::new(Mutex::new(Some(InnerByteStream::Bytes(bytes))))],
            }
        }

        pub fn flatten(byte_streams: Vec<ByteStream>) -> Self {
            let mut v = vec![];
            for bs in byte_streams {
                for ibs in bs.inner {
                    let oibs = ibs.lock();
                    if oibs.is_none() {
                        continue;
                    }
                    v.push(ibs.clone())
                }
            }

            Self { inner: v }
        }

        pub async fn collect(self) -> Result<AggregatedBytes, AwsByteStreamError> {
            let mut v = vec![];
            for ibs in self.inner {
                let mut ibs = ibs.lock();
                match ibs.take() {
                    Some(InnerByteStream::ByteStream(bs)) => {
                        let agg = bs.collect().await?;
                        let bytes = agg.into_bytes().to_vec();
                        let mut buf = SegmentedBuf::new();
                        buf.push(Bytes::from(bytes.clone()));
                        v.push(InnerAggregatedBytes::Madsim(buf));
                        *ibs = Some(InnerByteStream::Bytes(bytes));
                    }
                    Some(InnerByteStream::Bytes(bytes)) => {
                        let mut buf = SegmentedBuf::new();
                        buf.push(Bytes::from(bytes.clone()));
                        v.push(InnerAggregatedBytes::Madsim(buf));
                        *ibs = Some(InnerByteStream::Bytes(bytes));
                    }
                    None => {
                        continue;
                    }
                }
            }
            Ok(AggregatedBytes { inner: v })
        }

        pub fn extend(&mut self, other: ByteStream) {
            for ibs in other.inner {
                self.inner.push(ibs);
            }
        }

        pub async fn len(&self) -> usize {
            let mut l = 0;
            for ibs in self.inner.iter() {
                let mut ibs = ibs.lock();
                match ibs.take() {
                    Some(InnerByteStream::ByteStream(bs)) => {
                        let agg = bs.collect().await.expect("byte stream collect failed");
                        let bytes = agg.into_bytes().to_vec();
                        l += bytes.len();
                        *ibs = Some(InnerByteStream::Bytes(bytes));
                    }
                    Some(InnerByteStream::Bytes(bytes)) => {
                        l += bytes.len();
                        *ibs = Some(InnerByteStream::Bytes(bytes));
                    }
                    None => {
                        continue;
                    }
                }
            }
            l
        }

        pub async fn before(&self, mut end: usize) -> ByteStream {
            let mut v = vec![];
            for aibs in self.inner.iter() {
                if end == 0 {
                    break;
                }
                let mut ibs = aibs.lock();
                match ibs.take() {
                    Some(InnerByteStream::ByteStream(bs)) => {
                        let agg = bs.collect().await.expect("byte stream collect failed");
                        let bytes = agg.into_bytes().to_vec();
                        if bytes.len() <= end {
                            end -= bytes.len();
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                            v.push(aibs.clone())
                        } else {
                            v.push(Arc::new(Mutex::new(Some(InnerByteStream::Bytes(
                                bytes[..end].to_vec(),
                            )))));
                            end = 0;
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                        }
                    }
                    Some(InnerByteStream::Bytes(bytes)) => {
                        if bytes.len() <= end {
                            end -= bytes.len();
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                            v.push(aibs.clone())
                        } else {
                            v.push(Arc::new(Mutex::new(Some(InnerByteStream::Bytes(
                                bytes[..end].to_vec(),
                            )))));
                            end = 0;
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                        }
                    }
                    None => {
                        continue;
                    }
                }
            }
            ByteStream { inner: v }
        }

        pub async fn after(&self, mut begin: usize) -> ByteStream {
            let mut v = vec![];
            for aibs in self.inner.iter() {
                if begin == 0 {
                    v.push(aibs.clone());
                    continue;
                }
                let mut ibs = aibs.lock();
                match ibs.take() {
                    Some(InnerByteStream::ByteStream(bs)) => {
                        let agg = bs.collect().await.expect("byte stream collect failed");
                        let bytes = agg.into_bytes().to_vec();
                        if bytes.len() <= begin {
                            begin -= bytes.len();
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                        } else {
                            v.push(Arc::new(Mutex::new(Some(InnerByteStream::Bytes(
                                bytes[begin..].to_vec(),
                            )))));
                            begin = 0;
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                        }
                    }
                    Some(InnerByteStream::Bytes(bytes)) => {
                        if bytes.len() <= begin {
                            begin -= bytes.len();
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                        } else {
                            v.push(Arc::new(Mutex::new(Some(InnerByteStream::Bytes(
                                bytes[begin..].to_vec(),
                            )))));
                            begin = 0;
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                        }
                    }
                    None => {
                        continue;
                    }
                }
            }
            ByteStream { inner: v }
        }

        pub async fn between(&self, mut begin: usize, mut end: usize) -> ByteStream {
            let mut v = vec![];
            for aibs in self.inner.iter() {
                if end == 0 {
                    break;
                }
                let mut ibs = aibs.lock();
                match ibs.take() {
                    Some(InnerByteStream::ByteStream(bs)) => {
                        let agg = bs.collect().await.expect("byte stream collect failed");
                        let bytes = agg.into_bytes().to_vec();
                        if bytes.len() <= begin {
                            begin -= bytes.len();
                            end -= bytes.len();
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                        } else if bytes.len() >= end {
                            v.push(Arc::new(Mutex::new(Some(InnerByteStream::Bytes(
                                bytes[begin..end].to_vec(),
                            )))));
                            begin = 0;
                            end = 0;
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                        } else {
                            v.push(Arc::new(Mutex::new(Some(InnerByteStream::Bytes(
                                bytes[begin..].to_vec(),
                            )))));
                            begin = 0;
                            end -= bytes.len();
                        }
                    }
                    Some(InnerByteStream::Bytes(bytes)) => {
                        if bytes.len() <= begin {
                            begin -= bytes.len();
                            end -= bytes.len();
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                        } else if bytes.len() >= end {
                            v.push(Arc::new(Mutex::new(Some(InnerByteStream::Bytes(
                                bytes[begin..end].to_vec(),
                            )))));
                            begin = 0;
                            end = 0;
                            *ibs = Some(InnerByteStream::Bytes(bytes));
                        } else {
                            v.push(Arc::new(Mutex::new(Some(InnerByteStream::Bytes(
                                bytes[begin..].to_vec(),
                            )))));
                            begin = 0;
                            end -= bytes.len();
                        }
                    }
                    None => {
                        continue;
                    }
                }
            }
            ByteStream { inner: v }
        }
    }

    impl From<SdkBody> for ByteStream {
        fn from(inp: SdkBody) -> Self {
            ByteStream::new(inp)
        }
    }

    impl From<Bytes> for ByteStream {
        fn from(input: Bytes) -> Self {
            ByteStream::new(SdkBody::from(input))
        }
    }

    impl From<Vec<u8>> for ByteStream {
        fn from(input: Vec<u8>) -> Self {
            Self::from(Bytes::from(input))
        }
    }
}
