use std::mem::size_of;

use byteorder::{ByteOrder, LittleEndian};
use rand::Rng;

pub trait Record {
    fn new() -> Self;
    fn get_size(&self) -> u64;
    fn get_bytes(&self) -> Vec<u8>;
    fn from_bytes(&mut self, bytes: Vec<u8>) -> Result<(), std::io::Error>;
    fn from_string(&mut self, string: String) -> Result<(), std::io::Error>;
    fn from_random(&mut self) -> Result<(), std::io::Error>;
    fn print(&self);
}

pub struct IntRecord {
    // TODO: make it 15
    numbers: [u32; 16],
}

impl Record for IntRecord {
    fn new() -> Self {
        IntRecord { numbers: [0; 16] }
    }

    fn get_size(&self) -> u64 {
        return std::mem::size_of_val(&self.numbers) as u64;
    }

    fn get_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(size_of::<u32>() * self.numbers.len());

        for value in self.numbers {
            bytes.extend(&value.to_le_bytes());
        }

        bytes
    }

    fn from_bytes(&mut self, bytes: Vec<u8>) -> Result<(), std::io::Error> {
        if self.get_size() != (bytes.len() as u64) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Supplied data is not the same size as required to create the record",
            ));
        };

        LittleEndian::read_u32_into(&bytes, &mut self.numbers);
        Ok(())
    }

    fn from_string(&mut self, string: String) -> Result<(), std::io::Error> {
        let mut numbers: Vec<u32> = string
            .split_ascii_whitespace()
            .map(|s| s.parse::<u32>().unwrap_or(0u32))
            .collect::<Vec<u32>>();
        numbers.resize(self.numbers.len(), 0);

        self.numbers.copy_from_slice(&numbers);
        Ok(())
    }

    fn from_random(&mut self) -> Result<(), std::io::Error> {
        let mut rng = rand::thread_rng();
        self.numbers = rng.gen();
        Ok(())
    }

    fn print(&self) {
        println!("{:?}", self.numbers);
    }
}

#[cfg(test)]
mod tests {
    use std::mem::{size_of};

    use super::*;

    #[test]
    fn test_from_string() -> Result<(), std::io::Error> {
        let mut record: IntRecord = IntRecord::new();
        record.from_string("1 2 3 4 5 6 7".to_string())?;

        assert_eq!(
            record.numbers,
            [1u32, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        Ok(())
    }

    #[test]
    fn test_from_bytes() -> Result<(), std::io::Error> {
        let mut bytes: Vec<u8> = vec![0u8; 15*size_of::<u32>()];
        bytes[0] = 1;
        bytes[1] = 0;
        bytes[2] = 0;
        bytes[3] = 0;
        bytes[4] = 2;
        bytes[5] = 0;
        bytes[6] = 0;
        bytes[7] = 0;

        let mut record: IntRecord = IntRecord::new();
        record.from_bytes(bytes)?;

        assert_eq!(
            record.numbers,
            [1u32, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        Ok(())
    }
}
