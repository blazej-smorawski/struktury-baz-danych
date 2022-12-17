use byteorder::{ByteOrder, LittleEndian};
use std::fmt::{Debug, Display};

pub trait BTreeKey: Ord + Copy + Debug + Display {
    fn is_valid(&self) -> bool;
    fn invalidate(&mut self);
    fn to_bytes(&self) -> Vec<u8>;

    fn invalid() -> Self;
    fn from_bytes(bytes: &[u8]) -> Self;
    fn get_size() -> u64;
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct IntKey {
    pub value: i32,
}

impl BTreeKey for IntKey {
    fn is_valid(&self) -> bool {
        return self.value == i32::min_value();
    }

    fn invalidate(&mut self) {
        self.value = i32::min_value();
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![0u8; 4];
        LittleEndian::write_i32(&mut buf, self.value);
        buf
    }

    fn invalid() -> Self {
        IntKey {
            value: i32::min_value(),
        }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        IntKey {
            value: LittleEndian::read_i32(bytes),
        }
    }

    fn get_size() -> u64 {
        std::mem::size_of::<i32>() as u64
    }
}

impl Display for IntKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.value != i32::min_value() {
            write!(f, "{:<2}", format!("{}", self.value))
        } else {
            write!(f, "{:<2}", "*")
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_bytes() -> Result<(), std::io::Error> {
        let key = IntKey { value: 7 };

        assert_eq!(key.to_bytes(), [7u8, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn test_from_bytes() -> Result<(), std::io::Error> {
        let bytes: Vec<u8> = vec![7u8, 0, 0, 0];

        let key = IntKey::from_bytes(&bytes);

        assert_eq!(key.value, 7);
        Ok(())
    }
}
