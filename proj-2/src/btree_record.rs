use std::fmt::Display;

use crate::{btree_key::BTreeKey, bytes::Bytes};
use byteorder::{ByteOrder, LittleEndian};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct BTreeRecord<K: BTreeKey> {
    pub child_lba: Option<u64>,
    pub data_lba: u64,
    pub key: K,
}

impl<K: BTreeKey> Bytes for BTreeRecord<K> {
    fn invalid() -> Self {
        BTreeRecord::<K> {
            child_lba: None,
            key: K::invalid(),
            data_lba: 0,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let mut record = BTreeRecord::<K> {
            child_lba: None,
            key: K::from_bytes(&bytes[17..].to_vec()),
            data_lba: LittleEndian::read_u64(&bytes[9..9 + 8]),
        };

        if bytes[0] == 1 {
            record.child_lba = Some(LittleEndian::read_u64(&bytes[1..1 + 8]))
        }

        record
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; Self::get_size() as usize];

        if let Some(child_lba) = self.child_lba {
            bytes[0] = 1;
            LittleEndian::write_u64(&mut bytes[1..1 + 8], child_lba);
        } else {
            bytes[0] = 0;
            LittleEndian::write_u64(&mut bytes[1..1 + 8], 0)
        }

        LittleEndian::write_u64(&mut bytes[9..9 + 8], self.data_lba);
        bytes[17..].copy_from_slice(&self.key.to_bytes());

        bytes
    }

    fn get_size() -> u64 {
        1 + (2 * std::mem::size_of::<u64>()) as u64 + K::get_size()
    }
}

impl<K: BTreeKey> Display for BTreeRecord<K>
where
    K: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(child) = self.child_lba {
            write!(f, "{}.{:<4}", self.key, child)
        } else {
            write!(f, "{}.{:<4}", self.key, "*")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::btree_key::IntKey;

    use super::*;

    #[test]
    fn test_to_bytes() -> Result<(), std::io::Error> {
        let key = IntKey { value: 7 };
        let record = BTreeRecord {
            child_lba: Some(0xDEADBEEFAAAABBBB),
            data_lba: 0xFFEEFFEEFFEEFFEE,
            key: key,
        };

        let bytes = record.to_bytes();

        assert_eq!(
            bytes,
            [
                1u8, 0xBB, 0xBB, 0xAA, 0xAA, 0xEF, 0xBE, 0xAD, 0xDE, 0xEE, 0xFF, 0xEE, 0xFF, 0xEE,
                0xFF, 0xEE, 0xFF, 7, 0, 0, 0
            ]
        );

        Ok(())
    }

    #[test]
    fn test_from_bytes() -> Result<(), std::io::Error> {
        let key = IntKey { value: 7 };
        let record = BTreeRecord {
            child_lba: Some(0xDEADBEEFAAAABBBB),
            data_lba: 0xFFEEFFEEFFEEFFEE,
            key: key,
        };

        let bytes = vec![
            1u8, 0xBB, 0xBB, 0xAA, 0xAA, 0xEF, 0xBE, 0xAD, 0xDE, 0xEE, 0xFF, 0xEE, 0xFF, 0xEE,
            0xFF, 0xEE, 0xFF, 7, 0, 0, 0,
        ];

        let record_from_bytes = BTreeRecord::<IntKey>::from_bytes(&bytes);

        assert_eq!(record, record_from_bytes,);

        Ok(())
    }

    #[test]
    fn test_from_bytes_none() -> Result<(), std::io::Error> {
        let key = IntKey { value: 7 };
        let record = BTreeRecord {
            child_lba: None,
            data_lba: 0xFFEEFFEEFFEEFFEE,
            key: key,
        };

        let bytes = vec![
            0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0xEE, 0xFF, 0xEE, 0xFF, 0xEE, 0xFF, 0xEE, 0xFF, 7, 0, 0, 0,
        ];

        let record_from_bytes = BTreeRecord::<IntKey>::from_bytes(&bytes);

        assert_eq!(record, record_from_bytes,);

        Ok(())
    }

    #[test]
    fn test_invalid() -> Result<(), std::io::Error> {
        let key = IntKey { value: 7 };
        let record = BTreeRecord {
            child_lba: Some(0xDEADBEEFAAAABBBB),
            data_lba: 0xFFEEFFEEFFEEFFEE,
            key: key,
        };

        let invalid_record = BTreeRecord::<IntKey>::invalid();

        assert_ne!(record, invalid_record);

        Ok(())
    }

    #[test]
    fn test_partially_invalid() -> Result<(), std::io::Error> {
        let key = IntKey::invalid();
        let record = BTreeRecord {
            child_lba: Some(0xDEADBEEFAAAABBBB),
            data_lba: 0,
            key: key,
        };

        let invalid_record = BTreeRecord::<IntKey>::invalid();

        assert_ne!(record, invalid_record);

        Ok(())
    }
}
