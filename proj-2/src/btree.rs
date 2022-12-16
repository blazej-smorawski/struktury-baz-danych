use std::{cell::RefCell, rc::Rc, vec};

use crate::{
    btree_key::{BTreeKey, IntKey},
    btree_record::BTreeRecord,
    bytes::Bytes,
    device::BlockDevice,
    page::Page,
    record::Record,
};

pub struct BTree<K: BTreeKey, T: Record> {
    pub index_device: Rc<RefCell<BlockDevice>>,
    pub data_device: Rc<RefCell<BlockDevice>>,
    pub index_root: Page<BTreeRecord<K>>,
    pub working_page: Option<Page<BTreeRecord<K>>>,
    pub loaded_data: Vec<T>,
    pub degree: u64,
}

impl<K: BTreeKey, T: Record> BTree<K, T> {
    pub fn new(index_device: BlockDevice, data_device: BlockDevice) -> Self {
        let block_size = index_device.block_size;
        let record_size = BTreeRecord::<K>::get_size();
        let child_count: u64 = index_device.block_size / BTreeRecord::<K>::get_size();
        let degree = child_count / 2;
        let index_device = Rc::new(RefCell::new(index_device));
        let data_device = Rc::new(RefCell::new(data_device));

        let btree = BTree {
            index_device: index_device.clone(),
            data_device: data_device.clone(),
            index_root: Page::new(&index_device.clone(), 0, 0),
            working_page: None,
            loaded_data: vec![],
            degree: degree,
        };

        println!("BTree:\n\t- degree: {}\n\t- index block size: {}\n\t- record size: {}", degree, block_size, record_size);

        btree
    }

    fn search(&mut self, key: K) -> bool {
        let mut page_option = Some(&self.index_root);

        while let Some(page) = page_option {
            let records = &page.records;
            let found_record = records
                .iter()
                .find(|record| record.key == K::invalid() || record.key >= key);

            if let Some(record) = found_record {
                /*
                 * We found something interesting
                 */
                if record.key == key {
                    return true;
                }

                match record.child_lba {
                    Some(lba) => {
                        self.working_page = Some(Page::<BTreeRecord<K>>::new(
                            &self.index_device.clone(),
                            lba,
                            page.lba,
                        ));
                        page_option = self.working_page.as_ref();
                    }
                    None => page_option = None,
                }
            } else {
                /*
                 * Should not happen!
                 */
                panic!("No page to go into!")
            }
        }
        /*
         * We did not find anything
         */
        return false;
    }
}
