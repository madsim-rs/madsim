use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Debug, Display};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

/// A wrapper over `Vec<u8>` to represent bytes.
///
/// The bytes will be display and serialized in escape format.
/// see [`std::ascii::escape_default`].
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes(Vec<u8>);

impl Bytes {
    pub const fn new() -> Self {
        Bytes(Vec::new())
    }
}

impl Debug for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{self}\"")
    }
}

impl Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.escape_ascii())
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ParseBytesError {
    #[error("invalid hex")]
    InvalidHex,
    #[error("invalid byte")]
    InvalidByte,
    #[error("unexpected end of file")]
    UnexpectedEof,
}

impl FromStr for Bytes {
    type Err = ParseBytesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = Vec::with_capacity(s.len());
        let mut iter = s.bytes();
        while let Some(b) = iter.next() {
            match b {
                b'\\' => match iter.next().ok_or(ParseBytesError::UnexpectedEof)? {
                    b't' => bytes.push(b'\t'),
                    b'r' => bytes.push(b'\r'),
                    b'n' => bytes.push(b'\n'),
                    b'\'' => bytes.push(b'\''),
                    b'"' => bytes.push(b'"'),
                    b'\\' => bytes.push(b'\\'),
                    b'x' => {
                        let a = (iter.next().ok_or(ParseBytesError::UnexpectedEof)? as char)
                            .to_digit(16)
                            .ok_or(ParseBytesError::InvalidHex)?
                            as u8;
                        let b = (iter.next().ok_or(ParseBytesError::UnexpectedEof)? as char)
                            .to_digit(16)
                            .ok_or(ParseBytesError::InvalidHex)?
                            as u8;
                        bytes.push((a << 4) | b);
                    }
                    _ => return Err(ParseBytesError::InvalidByte),
                },
                0x20..=0x7e => bytes.push(b),
                _ => return Err(ParseBytesError::InvalidByte),
            }
        }
        Ok(Bytes(bytes))
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EscapeStrVisitor;
        impl<'de> de::Visitor<'de> for EscapeStrVisitor {
            type Value = Bytes;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "an escaped string")
            }

            fn visit_str<E>(self, data: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Bytes::from_str(data).map_err(de::Error::custom)
            }
        }
        deserializer.deserialize_str(EscapeStrVisitor)
    }
}

impl Deref for Bytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Bytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}
