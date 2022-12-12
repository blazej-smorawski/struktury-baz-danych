use std::vec;

use crate::{device::BlockDevice, record::Record, btree_key::BTreeKey, btree_record::BTreeRecord};

pub struct BTree<K: BTreeKey, T: Record> {
    pub index_device: BlockDevice,
    pub data_device: BlockDevice,
    pub loaded_index: Vec::<BTreeRecord<K>>,
    pub loaded_data: Vec::<T>
}

impl<'a, K: BTreeKey, T: Record> BTree<K, T> {
    fn new(index_device: BlockDevice, data_device: BlockDevice) -> Self {
        let mut btree = BTree { 
            index_device: index_device, 
            data_device: data_device,
            loaded_index: vec![],
            loaded_data: vec![]
        };

        let zeros = vec![0u8; btree.index_device.block_size as usize];
        btree.index_device.write(0, &zeros).expect("Could not write initial block into index device");

        let zeros = vec![0u8; btree.data_device.block_size as usize];
        btree.data_device.write(0, &zeros).expect("Could not write initial block into data device");

        btree
    }

    fn load_index_page(&mut self, lba: u64) {
        
    }
}