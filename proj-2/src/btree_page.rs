use std::cell::RefCell;
use std::{rc::Rc};

use crate::device::BlockDevice;

use crate::{btree_record::BTreeRecord, btree_key::BTreeKey, record::Record};

pub struct BTreePage<K: BTreeKey> {
    device: Rc<RefCell<BlockDevice>>,
    pub records: Vec<BTreeRecord<K>>,
    pub dirty: bool,
    pub lba: u64,
    pub parent_lba: u64,
}

impl<K: BTreeKey> BTreePage<K> {
    pub fn new(device: &Rc<RefCell<BlockDevice>>, lba: u64, parent_lba: u64) -> Self {
        let mut page = BTreePage::<K> {
            device: Rc::clone(device),
            records: Vec::<BTreeRecord<K>>::new(),
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
                    let len = BTreeRecord::<K>::get_size() as usize;
                    while off + len <= device.block_size as usize {
                        bytes[off..off + len].copy_from_slice(&BTreeRecord::<K>::invalid().to_bytes());
                        off += len;
                    }
                    //device.write(lba, &bytes).expect("Failed to write new block into index device!");
                    page.dirty = true;
                    bytes
                }
            };

            let mut off = 0 as usize;
            let len = BTreeRecord::<K>::get_size() as usize;
            while off + len <= bytes.len() {
                let record = BTreeRecord::<K>::from_bytes(&bytes[off..off + len]);
                if record == BTreeRecord::<K>::invalid() {
                    break;
                }
                page.records.push(record);
                off += len;
            }
        }

        page
    }
}

mod tests {
    use crate::btree_key::IntKey;
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

        let page = BTreePage::<IntKey>::new(&device, 0, 0);

        assert_eq!(page.records, Vec::<BTreeRecord<IntKey>>::new());

        Ok(())
    }

    #[test]
    fn test_new_one_record() -> Result<(), std::io::Error> {
        let block_size = 256;
        let mut device = BlockDevice::new("test_new_empty.txt".to_string(), block_size, true).unwrap();

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

        let page = BTreePage::<IntKey>::new(&device, 0, 0);

        let mut expected_records = Vec::<BTreeRecord<IntKey>>::new();
        expected_records.push(record);

        assert_ne!(page.records, Vec::<BTreeRecord<IntKey>>::new());
        assert_eq!(page.records, expected_records);

        Ok(())
    }
}