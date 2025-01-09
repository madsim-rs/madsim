use crate::error::KafkaError;
use crate::util::millis_to_epoch;
use std::marker::PhantomData;
use std::time::SystemTime;

/// A cheap conversion from a byte slice to typed data.
pub trait FromBytes {
    /// The error type that will be returned if the conversion fails.
    type Error;
    /// Tries to convert the provided byte slice into a different type.
    fn from_bytes(_: &[u8]) -> Result<&Self, Self::Error>;
}

impl FromBytes for [u8] {
    type Error = ();
    fn from_bytes(bytes: &[u8]) -> Result<&Self, Self::Error> {
        Ok(bytes)
    }
}

impl FromBytes for str {
    type Error = std::str::Utf8Error;
    fn from_bytes(bytes: &[u8]) -> Result<&Self, Self::Error> {
        std::str::from_utf8(bytes)
    }
}

/// A cheap conversion from typed data to a byte slice.
pub trait ToBytes {
    /// Converts the provided data to bytes.
    fn to_bytes(&self) -> &[u8];
}

impl ToBytes for [u8] {
    fn to_bytes(&self) -> &[u8] {
        self
    }
}

impl<const N: usize> ToBytes for [u8; N] {
    fn to_bytes(&self) -> &[u8] {
        self
    }
}

impl ToBytes for str {
    fn to_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl ToBytes for Vec<u8> {
    fn to_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}

impl ToBytes for String {
    fn to_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<T: ToBytes> ToBytes for &T {
    fn to_bytes(&self) -> &[u8] {
        (*self).to_bytes()
    }
}

impl ToBytes for () {
    fn to_bytes(&self) -> &[u8] {
        &[]
    }
}

/// A generic representation of Kafka message headers.
///
/// This trait represents readable message headers. Headers are key-value pairs
/// that can be sent alongside every message. Only read-only methods are
/// provided by this trait, as the underlying storage might not allow
/// modification.
pub trait Headers {
    /// Returns the number of contained headers.
    fn count(&self) -> usize;

    /// Gets the specified header, where the first header corresponds to
    /// index 0.
    ///
    /// Panics if the index is out of bounds.
    fn get(&self, idx: usize) -> Header<'_, &[u8]> {
        self.try_get(idx).unwrap_or_else(|| {
            panic!(
                "headers index out of bounds: the count is {} but the index is {}",
                self.count(),
                idx,
            )
        })
    }

    /// Like [`Headers::get`], but the value of the header will be converted
    /// to the specified type.
    ///
    /// Panics if the index is out of bounds.
    fn get_as<V>(&self, idx: usize) -> Result<Header<'_, &V>, V::Error>
    where
        V: FromBytes + ?Sized,
    {
        self.try_get_as(idx).unwrap_or_else(|| {
            panic!(
                "headers index out of bounds: the count is {} but the index is {}",
                self.count(),
                idx,
            )
        })
    }

    /// Like [`Headers::get`], but returns an option if the header is out of
    /// bounds rather than panicking.
    fn try_get(&self, idx: usize) -> Option<Header<'_, &[u8]>>;

    /// Like [`Headers::get`], but returns an option if the header is out of
    /// bounds rather than panicking.
    fn try_get_as<V>(&self, idx: usize) -> Option<Result<Header<'_, &V>, V::Error>>
    where
        V: FromBytes + ?Sized,
    {
        self.try_get(idx).map(|header| header.parse())
    }

    /// Iterates over all headers in order.
    fn iter(&self) -> HeadersIter<'_, Self>
    where
        Self: Sized,
    {
        HeadersIter {
            headers: self,
            index: 0,
        }
    }
}

/// A generic representation of a Kafka message.
///
/// Only read-only methods are provided by this trait, as the underlying storage
/// might not allow modification.
pub trait Message {
    /// The type of headers that this message contains.
    type Headers: Headers;

    /// Returns the key of the message, or `None` if there is no key.
    fn key(&self) -> Option<&[u8]>;

    /// Returns the payload of the message, or `None` if there is no payload.
    fn payload(&self) -> Option<&[u8]>;

    /// Returns the source topic of the message.
    fn topic(&self) -> &str;

    /// Returns the partition number where the message is stored.
    fn partition(&self) -> i32;

    /// Returns the offset of the message within the partition.
    fn offset(&self) -> i64;

    /// Returns the message timestamp.
    fn timestamp(&self) -> Timestamp;

    /// Returns the headers of the message, or `None` if there are no headers.
    fn headers(&self) -> Option<&Self::Headers>;
}

/// Timestamp of a Kafka message.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Timestamp {
    /// Timestamp not available.
    NotAvailable,
    /// Message creation time.
    CreateTime(i64),
    /// Log append time.
    LogAppendTime(i64),
}

impl Timestamp {
    /// Convert the timestamp to milliseconds since epoch.
    pub fn to_millis(self) -> Option<i64> {
        match self {
            Self::NotAvailable | Self::CreateTime(-1) | Self::LogAppendTime(-1) => None,
            Self::CreateTime(t) | Self::LogAppendTime(t) => Some(t),
        }
    }

    /// Creates a new `Timestamp::CreateTime` representing the current time.
    pub fn now() -> Timestamp {
        Timestamp::from(SystemTime::now())
    }
}

impl From<i64> for Timestamp {
    fn from(system_time: i64) -> Timestamp {
        Timestamp::CreateTime(system_time)
    }
}

impl From<SystemTime> for Timestamp {
    fn from(system_time: SystemTime) -> Timestamp {
        Timestamp::CreateTime(millis_to_epoch(system_time))
    }
}

/// A Kafka message that owns its backing data.
#[derive(Debug, Clone)]
pub struct OwnedMessage {
    pub(crate) payload: Option<Vec<u8>>,
    pub(crate) key: Option<Vec<u8>>,
    pub(crate) topic: String,
    pub(crate) timestamp: Timestamp,
    pub(crate) partition: i32,
    pub(crate) offset: i64,
    pub(crate) headers: Option<OwnedHeaders>,
}

impl OwnedMessage {
    /// Returns the estimate size in bytes.
    pub(crate) fn size(&self) -> usize {
        let mut size = 10;
        if let Some(ref payload) = self.payload {
            size += payload.len();
        }
        if let Some(ref key) = self.key {
            size += key.len();
        }
        size
    }

    pub(crate) fn borrow(self) -> BorrowedMessage<'static> {
        BorrowedMessage {
            msg: self,
            _mark: PhantomData,
        }
    }
}

impl Message for OwnedMessage {
    type Headers = OwnedHeaders;

    fn key(&self) -> Option<&[u8]> {
        match self.key {
            Some(ref k) => Some(k.as_slice()),
            None => None,
        }
    }

    fn payload(&self) -> Option<&[u8]> {
        self.payload.as_deref()
    }

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }

    fn partition(&self) -> i32 {
        self.partition
    }

    fn offset(&self) -> i64 {
        self.offset
    }

    fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    fn headers(&self) -> Option<&Self::Headers> {
        self.headers.as_ref()
    }
}

/// The result of a message production.
pub type DeliveryResult<'a> = Result<BorrowedMessage<'a>, (KafkaError, BorrowedMessage<'a>)>;

/// A zero-copy Kafka message.
pub struct BorrowedMessage<'a> {
    msg: OwnedMessage,
    _mark: PhantomData<&'a ()>,
}

impl Message for BorrowedMessage<'_> {
    type Headers = BorrowedHeaders;

    fn key(&self) -> Option<&[u8]> {
        self.msg.key()
    }

    fn payload(&self) -> Option<&[u8]> {
        self.msg.payload()
    }

    fn topic(&self) -> &str {
        self.msg.topic()
    }

    fn partition(&self) -> i32 {
        self.msg.partition()
    }

    fn offset(&self) -> i64 {
        self.msg.offset()
    }

    fn timestamp(&self) -> Timestamp {
        self.msg.timestamp()
    }

    fn headers(&self) -> Option<&Self::Headers> {
        self.msg
            .headers()
            .map(|h| unsafe { std::mem::transmute(h) })
    }
}

impl BorrowedMessage<'_> {
    pub fn detach(&self) -> OwnedMessage {
        self.msg.clone()
    }
}

/// A zero-copy collection of Kafka message headers.
#[repr(transparent)]
pub struct BorrowedHeaders(OwnedHeaders);

impl BorrowedHeaders {
    /// Clones the content of `BorrowedHeaders` and returns an [`OwnedHeaders`]
    /// that can outlive the consumer.
    ///
    /// This operation requires memory allocation and can be expensive.
    pub fn detach(&self) -> OwnedHeaders {
        self.0.clone()
    }
}

impl Headers for BorrowedHeaders {
    fn count(&self) -> usize {
        self.0.count()
    }

    fn try_get(&self, idx: usize) -> Option<Header<'_, &[u8]>> {
        self.0.try_get(idx)
    }
}

/// A collection of Kafka message headers that owns its backing data.
#[derive(Debug, Clone)]
pub struct OwnedHeaders {
    headers: Vec<(String, Vec<u8>)>,
}

impl Headers for OwnedHeaders {
    fn count(&self) -> usize {
        self.headers.len()
    }

    fn try_get(&self, idx: usize) -> Option<Header<'_, &[u8]>> {
        self.headers.get(idx).map(|(k, v)| Header {
            key: k.as_str(),
            value: Some(v.as_slice()),
        })
    }
}

/// A Kafka message header.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Header<'a, V> {
    /// The header's key.
    pub key: &'a str,
    /// The header's value.
    pub value: Option<V>,
}

impl<'a> Header<'a, &'a [u8]> {
    fn parse<V>(&self) -> Result<Header<'a, &'a V>, V::Error>
    where
        V: FromBytes + ?Sized,
    {
        Ok(Header {
            key: self.key,
            value: self.value.map(V::from_bytes).transpose()?,
        })
    }
}

/// An iterator over [`Headers`].
pub struct HeadersIter<'a, H> {
    headers: &'a H,
    index: usize,
}

impl<'a, H> Iterator for HeadersIter<'a, H>
where
    H: Headers,
{
    type Item = Header<'a, &'a [u8]>;

    fn next(&mut self) -> Option<Header<'a, &'a [u8]>> {
        if self.index < self.headers.count() {
            let item = self.headers.get(self.index);
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}
