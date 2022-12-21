use crate::bytes::Bytes;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Pair<K: Bytes, T: Bytes> {
    pub key: K,
    pub value: T
} 

impl<K: Bytes, T: Bytes> Bytes for Pair<K, T> {
    fn invalid() -> Self {
        Pair { key: K::invalid(), value: T::invalid() }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Pair { 
            key: K::from_bytes(&bytes[0 .. K::get_size() as usize]), 
            value: T::from_bytes(&bytes[K::get_size() as usize .. (K::get_size() + T::get_size()) as usize]) 
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.key.to_bytes();
        bytes.append(&mut self.value.to_bytes());
        bytes
    }

    fn get_size() -> u64 {
        K::get_size() + T::get_size()
    }
}

#[cfg(test)]
mod tests {
    use crate::{btree_key::IntKey, record::{IntRecord, Record}};

    use super::*;

    #[test]
    fn test_to_bytes() -> Result<(), std::io::Error> {
        let pair = Pair {
            key: IntKey{value: 5},
            value: IntRecord::from_string("1 2 3 4 5".to_string()).unwrap()
        };

        assert_eq!(pair.to_bytes(), vec![5,0,0,0,1,0,0,0,2,0,0,0,3,0,0,0,4,0,0,0,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]);

        Ok(())
    }
}