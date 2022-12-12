use std::vec;

use crate::{btree_key::BTreeKey, btree_record::BTreeRecord, device::BlockDevice, record::Record};

pub struct BTree<K: BTreeKey, T: Record> {
    pub index_device: BlockDevice,
    pub data_device: BlockDevice,
    pub loaded_index: Vec<BTreeRecord<K>>,
    pub loaded_data: Vec<T>,
}

impl<K: BTreeKey, T: Record> BTree<K, T> {
    pub fn new(index_device: BlockDevice, data_device: BlockDevice) -> Self {
        let mut btree = BTree {
            index_device: index_device,
            data_device: data_device,
            loaded_index: vec![],
            loaded_data: vec![],
        };

        let zeros = vec![0u8; btree.index_device.block_size as usize];
        btree
            .index_device
            .write(0, &zeros)
            .expect("Could not write initial block into index device");

        let zeros = vec![0u8; btree.data_device.block_size as usize];
        btree
            .data_device
            .write(0, &zeros)
            .expect("Could not write initial block into data device");

        btree
    }

    fn load_index_page(&mut self, lba: u64) {
        let block_size = self.index_device.block_size as usize;
        let mut buf = vec![0u8; block_size];
        let read_result = self.index_device.read(&mut buf, lba);

        self.loaded_index.clear();

        match read_result {
            Ok(_) => (),
            Err(_) => {
                // Fill buf the block with invalid records, ready to be overwriten
                let off = 0 as usize;
                let len = BTreeRecord::<K>::get_size() as usize;
                while off + len <= block_size {
                    buf[off..off + len].copy_from_slice(&BTreeRecord::<K>::invalid().to_bytes())
                }
                self.index_device.write(lba, &buf).expect("Failed to write new block into index device!");
                return;
            }
        }
    }
}
