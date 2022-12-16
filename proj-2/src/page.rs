use std::cell::RefCell;
use std::{rc::Rc};

use crate::bytes::Bytes;
use crate::device::BlockDevice;

pub struct Page<K: Bytes> {
    device: Rc<RefCell<BlockDevice>>,
    pub records: Vec<Box<K>>,
    pub dirty: bool,
    pub lba: u64,
    pub parent_lba: u64,
}

impl<K: Bytes> Page<K> {
    pub fn new(device: &Rc<RefCell<BlockDevice>>, lba: u64, parent_lba: u64) -> Self {
        let mut page = Page::<K> {
            device: Rc::clone(device),
            records: Vec::<Box<K>>::new(),
            dirty: false,
            lba: lba,
            parent_lba: parent_lba, 
        };

        {
            let mut device = page.device.borrow_mut();
            let read_result = device.read(lba);

            let bytes = match read_result {
                Ok(bytes) => bytes,
                Err(_) => {
                    // Fill buf the block with invalid records, ready to be overwritten
                    let mut bytes = vec![0u8; device.block_size as usize];
                    let mut off = 0 as usize;
                    let len = K::get_size() as usize;
                    while off + len <= device.block_size as usize {
                        bytes[off..off + len].copy_from_slice(&K::invalid().to_bytes());
                        off += len;
                    }
                    //device.write(lba, &bytes).expect("Failed to write new block into index device!");
                    page.dirty = true;
                    bytes
                }
            };

            let mut off = 0 as usize;
            let len = K::get_size() as usize;
            while off + len <= bytes.len() {
                let record = K::from_bytes(&bytes[off..off + len]);
                if record == K::invalid() {
                    break;
                }
                page.records.push(Box::new(record));
                off += len;
            }
        }

        page
    }
}

impl<K: Bytes> Drop for Page<K> {
    fn drop(&mut self) {
        if self.dirty {
            let mut device = self.device.borrow_mut();
            
            let mut bytes = vec![0u8; device.block_size as usize];
            let mut off = 0 as usize;
            let len = K::get_size() as usize;

            for record in &self.records {
                bytes[off..off + len].copy_from_slice(&record.to_bytes());
                off += len;
            }

            // Fill rest with invalid records
            while off + len <= device.block_size as usize {
                bytes[off..off + len].copy_from_slice(&K::invalid().to_bytes());
                off += len;
            }

            device.write(self.lba, &bytes).expect("Could not write into device on flush!");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::btree_key::IntKey;
    use crate::{btree_record::BTreeRecord};
    use crate::device::BlockDevice;
    
    use super::*;

    #[test]
    fn test_new_empty() -> Result<(), std::io::Error> {
        let block_size = 256;
        let mut device = BlockDevice::new("test_new_empty.txt".to_string(), block_size, true).unwrap();

        let mut bytes = vec![0u8; block_size as usize];
        let mut off = 0 as usize;
        let len = BTreeRecord::<IntKey>::get_size() as usize;
        while off + len <= block_size as usize {
            bytes[off..off + len].copy_from_slice(&BTreeRecord::<IntKey>::invalid().to_bytes());
            off += len;
        }

        device.write(0, &bytes).unwrap();
        let device = Rc::new(RefCell::new(device));

        let page = Page::<BTreeRecord<IntKey>>::new(&device, 0, 0);

        assert_eq!(page.records, Vec::<Box<BTreeRecord<IntKey>>>::new());

        Ok(())
    }

    #[test]
    fn test_new_one_record() -> Result<(), std::io::Error> {
        let block_size = 256;
        let mut device = BlockDevice::new("test_new_one_record.txt".to_string(), block_size, true).unwrap();

        let mut bytes = vec![0u8; block_size as usize];
        let mut off = 0 as usize;
        let len = BTreeRecord::<IntKey>::get_size() as usize;
        while off + len <= block_size as usize {
            bytes[off..off + len].copy_from_slice(&BTreeRecord::<IntKey>::invalid().to_bytes());
            off += len;
        }

        let key = IntKey { value: 7 };
        let record = BTreeRecord {
            child_lba: Some(0xDEADBEEFAAAABBBB),
            data_lba: 0xFFEEFFEEFFEEFFEE,
            key: key,
        };
        bytes[0 .. BTreeRecord::<IntKey>::get_size() as usize].copy_from_slice(&record.to_bytes());

        device.write(0, &bytes).unwrap();
        let device = Rc::new(RefCell::new(device));

        let page = Page::<BTreeRecord<IntKey>>::new(&device, 0, 0);

        let mut expected_records = Vec::<Box<BTreeRecord<IntKey>>>::new();
        expected_records.push(Box::new(record));

        assert_ne!(page.records, Vec::<Box<BTreeRecord<IntKey>>>::new());
        assert_eq!(page.records, expected_records);

        Ok(())
    }
}