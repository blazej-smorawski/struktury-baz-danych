use std::{vec, rc::Rc, cell::RefCell};

use crate::{btree_key::BTreeKey, btree_record::BTreeRecord, device::BlockDevice, record::Record};

pub struct BTree<K: BTreeKey, T: Record> {
    pub index_device: Rc<RefCell<BlockDevice>>,
    pub data_device: Rc<RefCell<BlockDevice>>,
    pub loaded_index: Vec<BTreeRecord<K>>,
    pub loaded_data: Vec<T>,
}

impl<K: BTreeKey, T: Record> BTree<K, T> {
    pub fn new(index_device: BlockDevice, data_device: BlockDevice) -> Self {
        let btree = BTree {
            index_device: Rc::new(RefCell::new(index_device)),
            data_device: Rc::new(RefCell::new(data_device)),
            loaded_index: vec![],
            loaded_data: vec![],
        };

        btree
    }

    fn load_index_page(&mut self, lba: u64) {
        
    }
}
