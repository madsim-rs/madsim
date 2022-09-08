use std::marker::PhantomData;

/// A cheap conversion from typed data to a byte slice.
///
/// Given some data, returns the byte representation of that data.
/// No copy of the data should be performed.
///
/// See also the [`FromBytes`] trait.
pub trait ToBytes {
    /// Converts the provided data to bytes.
    fn to_bytes(&self) -> &[u8];
}

impl ToBytes for [u8] {
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

/// A generic representation of a Kafka message.
///
/// Only read-only methods are provided by this trait, as the underlying storage
/// might not allow modification.
pub trait Message {
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

    // /// Returns the message timestamp.
    // fn timestamp(&self) -> Timestamp;
}

/// A zero-copy Kafka message.
pub struct BorrowedMessage<'a> {
    _owner: PhantomData<&'a u8>,
}

impl<'a> Message for BorrowedMessage<'a> {
    fn key(&self) -> Option<&[u8]> {
        todo!()
    }

    fn payload(&self) -> Option<&[u8]> {
        todo!()
    }

    fn topic(&self) -> &str {
        todo!()
    }

    fn partition(&self) -> i32 {
        todo!()
    }

    fn offset(&self) -> i64 {
        todo!()
    }
}

/// A collection of Kafka message headers that owns its backing data.
#[derive(Debug)]
pub struct OwnedHeaders {}
