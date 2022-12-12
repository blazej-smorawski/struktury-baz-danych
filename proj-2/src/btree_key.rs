use byteorder::{LittleEndian, ByteOrder};

pub trait BTreeKey: Ord + Copy {
    fn from_bytes(bytes: &Vec<u8>) -> Self;
    fn to_bytes(&self) -> Vec<u8>;
    fn get_size() -> u64;
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct IntKey {
    pub value: i32
}

impl BTreeKey for IntKey {
    fn from_bytes(bytes: &Vec<u8>) -> Self {
        IntKey {
            value: LittleEndian::read_i32(bytes)
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![0u8; 4];
        LittleEndian::write_i32(&mut buf, self.value);
        buf
    }

    fn get_size() -> u64 {
        std::mem::size_of::<i32>() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_bytes() -> Result<(), std::io::Error> {
        let key = IntKey {value:7};

        assert_eq!(
            key.to_bytes(),
            [7u8, 0, 0, 0]
        );
        Ok(())
    }

    #[test]
    fn test_from_bytes() -> Result<(), std::io::Error> {
        let bytes: Vec<u8> = vec![7u8, 0 , 0, 0];

        let key = IntKey::from_bytes(&bytes);

        assert_eq!(
            key.value,
            7
        );
        Ok(())
    }
}
