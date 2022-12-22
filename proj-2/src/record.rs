use std::{cmp::Ordering, mem::size_of, fmt::Display};

use byteorder::{ByteOrder, LittleEndian};
use primes::is_prime;
use rand::Rng;

use crate::bytes::Bytes;

pub trait Record: Bytes + Ord + Copy + Display {
    fn new() -> Self;
    fn from_string(string: String) -> Result<Self, std::io::Error>;
    fn from_random() -> Result<Self, std::io::Error>;
    fn print(&self);
}

#[derive(Copy, Clone, Debug)]
pub struct IntRecord {
    numbers: [u32; 15],
}

impl IntRecord {
    fn get_primes(&self) -> u32 {
        let mut primes: u32 = 0;
        
        for num in self.numbers {
            if is_prime(num as u64) {
                primes += 1;
            }
        }
        primes
    }
}

impl Record for IntRecord {
    fn new() -> Self {
        IntRecord { numbers: [0; 15] }
    }

    fn from_string(string: String) -> Result<Self, std::io::Error> {
        let mut record = IntRecord::new();

        let mut numbers: Vec<u32> = string
            .split_ascii_whitespace()
            .map(|s| s.parse::<u32>().unwrap_or(0u32))
            .collect::<Vec<u32>>();
        numbers.resize(record.numbers.len(), 0);

        record.numbers.copy_from_slice(&numbers);
        Ok(record)
    }

    fn from_random() -> Result<Self, std::io::Error> {
        let mut record = IntRecord::new();

        let mut rng = rand::thread_rng();
        for number in &mut record.numbers {
            *number = rng.gen_range(1..10);
        }
        Ok(record)
    }

    fn print(&self) {
        println!("{:?} <=> {}", self.numbers, self.get_primes());
    }
}

impl Bytes for IntRecord {
    fn invalid() -> Self {
        Self::new()
    }

    fn get_size() -> u64 {
        //return std::mem::size_of::<u32>() as u64 * 15;
        (std::mem::size_of::<u32>() * 15) as u64
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(size_of::<u32>() * self.numbers.len());

        for value in self.numbers {
            bytes.extend(&value.to_le_bytes());
        }

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        if Self::get_size() != (bytes.len() as u64) {
            panic!("Supplied data is not the same size as required to create the record");
        };

        let mut record = IntRecord::new();

        LittleEndian::read_u32_into(&bytes, &mut record.numbers);

        record
    }
}

impl Ord for IntRecord {
    fn cmp(&self, other: &Self) -> Ordering {
        let primes: u32 = self.get_primes();
        let other_primes: u32 = other.get_primes();

        primes.cmp(&other_primes)
    }
}

impl PartialOrd for IntRecord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for IntRecord {}

impl PartialEq for IntRecord {
    fn eq(&self, other: &Self) -> bool {
        let mut primes: u32 = 0;
        let mut other_primes: u32 = 0;

        for num in self.numbers {
            if is_prime(num as u64) {
                primes += 1;
            }
        }
        for num in other.numbers {
            if is_prime(num as u64) {
                other_primes += 1;
            }
        }
        primes == other_primes
    }
}

impl Display for IntRecord{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} <=> {}", self.numbers, self.get_primes())
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;

    #[test]
    fn test_from_string() -> Result<(), std::io::Error> {
        let record= IntRecord::from_string("1 2 3 4 5 6 7".to_string())?;

        assert_eq!(
            record.numbers,
            [1u32, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        Ok(())
    }

    #[test]
    fn test_from_bytes() -> Result<(), std::io::Error> {
        let mut bytes: Vec<u8> = vec![0u8; 15 * size_of::<u32>()];
        bytes[0] = 1;
        bytes[1] = 0;
        bytes[2] = 0;
        bytes[3] = 0;
        bytes[4] = 2;
        bytes[5] = 0;
        bytes[6] = 0;
        bytes[7] = 0;

        let record = IntRecord::from_bytes(&bytes);

        assert_eq!(
            record.numbers,
            [1u32, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        Ok(())
    }
}
