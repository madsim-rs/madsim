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

impl<'a, T: ToBytes> ToBytes for &'a T {
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
pub trait Headers {
    /// Returns the number of contained headers.
    fn count(&self) -> usize;

    /// Gets the specified header, where the first header corresponds to index
    /// 0. If the index is out of bounds, returns `None`.
    fn get(&self, idx: usize) -> Option<(&str, &[u8])>;

    /// Like [`Headers::get`], but the value of the header will be converted to
    /// the specified type. If the conversion fails, returns an error.
    fn get_as<V: FromBytes + ?Sized>(&self, idx: usize) -> Option<(&str, Result<&V, V::Error>)> {
        self.get(idx)
            .map(|(name, value)| (name, V::from_bytes(value)))
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

/// A zero-copy collection of Kafka message headers.
#[repr(transparent)]
pub struct BorrowedHeaders(OwnedHeaders);

impl Headers for BorrowedHeaders {
    fn count(&self) -> usize {
        self.0.count()
    }

    fn get(&self, idx: usize) -> Option<(&str, &[u8])> {
        self.0.get(idx)
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

    fn get(&self, idx: usize) -> Option<(&str, &[u8])> {
        self.headers
            .get(idx)
            .map(|(k, v)| (k.as_str(), v.as_slice()))
    }
}
