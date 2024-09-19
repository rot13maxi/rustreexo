use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::fmt::Display;
use std::ops::Deref;
use std::str::FromStr;

#[cfg(feature = "with-serde")]
use serde::Deserialize;
#[cfg(feature = "with-serde")]
use serde::Serialize;
use sha2::{Digest, Sha512_256};

#[derive(Eq, PartialEq, Copy, Clone, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "with-serde", derive(Serialize, Deserialize))]
#[derive(Default)]
pub enum NodeHash {
    #[default]
    Empty,
    Placeholder,
    Some([u8; 32]),
}

impl Deref for NodeHash {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        match self {
            NodeHash::Some(ref inner) => inner,
            _ => &[0; 32],
        }
    }
}

impl Display for NodeHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        if let NodeHash::Some(ref inner) = self {
            for byte in inner.iter() {
                write!(f, "{:02x}", byte)?;
            }
            Ok(())
        } else {
            write!(f, "empty")
        }
    }
}

impl Debug for NodeHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        if let NodeHash::Some(ref inner) = self {
            for byte in inner.iter() {
                write!(f, "{:02x}", byte)?;
            }
            Ok(())
        } else {
            write!(f, "empty")
        }
    }
}

impl From<[u8; 32]> for NodeHash {
    fn from(hash: [u8; 32]) -> Self {
        NodeHash::Some(hash)
    }
}

impl From<&[u8; 32]> for NodeHash {
    fn from(hash: &[u8; 32]) -> Self {
        NodeHash::Some(*hash)
    }
}

#[cfg(test)]
impl TryFrom<&str> for NodeHash {
    type Error = hex::FromHexError;
    fn try_from(hash: &str) -> Result<Self, Self::Error> {
        if hash == "0000000000000000000000000000000000000000000000000000000000000000" {
            return Ok(NodeHash::Empty);
        }
        let hash = hex::decode(hash)?;
        Ok(NodeHash::Some(hash.try_into().unwrap()))
    }
}

#[cfg(not(test))]
impl TryFrom<&str> for NodeHash {
    type Error = hex::FromHexError;
    fn try_from(hash: &str) -> Result<Self, Self::Error> {
        let hash = hex::decode(hash)?;
        Ok(NodeHash::Some(hash.try_into().unwrap()))
    }
}

impl From<&[u8]> for NodeHash {
    fn from(hash: &[u8]) -> Self {
        let mut inner = [0; 32];
        inner.copy_from_slice(hash);
        NodeHash::Some(inner)
    }
}

impl FromStr for NodeHash {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NodeHash::try_from(s)
    }
    type Err = hex::FromHexError;
}

impl NodeHash {
    pub fn is_empty(&self) -> bool {
        matches!(self, NodeHash::Empty)
    }

    pub fn new(inner: [u8; 32]) -> Self {
        NodeHash::Some(inner)
    }

    pub fn empty() -> Self {
        NodeHash::Empty
    }

    pub fn parent_hash(left: &NodeHash, right: &NodeHash) -> NodeHash {
        let mut hasher = Sha512_256::new();
        hasher.update(&**left);
        hasher.update(&**right);
        let result = hasher.finalize();
        NodeHash::Some(result.into())
    }

    pub const fn placeholder() -> Self {
        NodeHash::Placeholder
    }

    pub(super) fn write<W>(&self, writer: &mut W) -> std::io::Result<()>
    where
        W: std::io::Write,
    {
        match self {
            Self::Empty => writer.write_all(&[0]),
            Self::Placeholder => writer.write_all(&[1]),
            Self::Some(hash) => {
                writer.write_all(&[2])?;
                writer.write_all(hash)
            }
        }
    }

    pub(super) fn read<R>(reader: &mut R) -> std::io::Result<Self>
    where
        R: std::io::Read,
    {
        let mut tag = [0];
        reader.read_exact(&mut tag)?;
        match tag {
            [0] => Ok(Self::Empty),
            [1] => Ok(Self::Placeholder),
            [2] => {
                let mut hash = [0; 32];
                reader.read_exact(&mut hash)?;
                Ok(Self::Some(hash))
            }
            [_] => {
                let err = std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unexpected tag for NodeHash",
                );
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::NodeHash;
    use crate::accumulator::util::hash_from_u8;

    #[test]
    fn test_parent_hash() {
        let hash1 = hash_from_u8(0);
        let hash2 = hash_from_u8(1);

        let parent_hash = NodeHash::parent_hash(&hash1, &hash2);
        assert_eq!(
            parent_hash.to_string().as_str(),
            "02242b37d8e851f1e86f46790298c7097df06893d6226b7c1453c213e91717de"
        );
    }

    #[test]
    fn test_hash_from_str() {
        let hash =
            NodeHash::from_str("6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d")
                .unwrap();
        assert_eq!(hash, hash_from_u8(0));
    }

    #[test]
    fn test_empty_hash() {
        let hash =
            NodeHash::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();
        assert_eq!(hash, NodeHash::empty());
    }
}
