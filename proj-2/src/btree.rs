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
                 * No records left to analyze
                 */
                break
            }
        }
        /*
         * We did not find anything
         */
        return false;
    }
}

#[cfg(test)]
mod tests {
    use crate::{btree_key::IntKey, device, record::IntRecord};

    use super::*;

    #[test]
    fn test_search() -> Result<(), std::io::Error> {
        let block_size = 21*4; // t = 2

        {
            let device = BlockDevice::new("test_search.hex".to_string(), block_size, true).unwrap();
            let device = Rc::new(RefCell::new(device));
            let mut root_page = Page::<BTreeRecord<IntKey>>::new(&device.clone(), 0, 0);
            let mut child1_page = Page::<BTreeRecord<IntKey>>::new(&device.clone(), 1, 0);
            let mut child2_page = Page::<BTreeRecord<IntKey>>::new(&device.clone(), 2, 0);
            //let child3_page = Page::<BTreeRecord<IntKey>>::new(&device.clone(), 0, 0);
            //let child4_page = Page::<BTreeRecord<IntKey>>::new(&device.clone(), 0, 0);

            let record1 = BTreeRecord::<IntKey> {
                child_lba: Some(1),
                key: IntKey { value: 10 },
                data_lba: 0,
            };
            let record2 = BTreeRecord::<IntKey> {
                child_lba: Some(2),
                key: IntKey::invalid(),
                data_lba: 0,
            };
            root_page.records.push(Box::new(record1));
            root_page.records.push(Box::new(record2));

            let record1 = BTreeRecord::<IntKey> {
                child_lba: None,
                key: IntKey { value: 4 },
                data_lba: 0,
            };
            let record2 = BTreeRecord::<IntKey> {
                child_lba: None,
                key: IntKey { value: 7 },
                data_lba: 0,
            };
            child1_page.records.push(Box::new(record1));
            child1_page.records.push(Box::new(record2));

            let record1 = BTreeRecord::<IntKey> {
                child_lba: None,
                key: IntKey { value: 12 },
                data_lba: 0,
            };
            let record2 = BTreeRecord::<IntKey> {
                child_lba: None,
                key: IntKey { value: 20 },
                data_lba: 0,
            };
            let record3 = BTreeRecord::<IntKey> {
                child_lba: None,
                key: IntKey::invalid(),
                data_lba: 0,
            };
            child2_page.records.push(Box::new(record1));
            child2_page.records.push(Box::new(record2));
            child2_page.records.push(Box::new(record3));
        }

        let device = BlockDevice::new("test_search.hex".to_string(), block_size, false).unwrap();
        let mut data_device = BlockDevice::new("test_search_data.hex".to_string(), block_size, false).unwrap();
        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        assert_eq!(btree.search(IntKey{value: 20}), true);
        assert_eq!(btree.search(IntKey{value: 10}), true);
        assert_eq!(btree.search(IntKey{value: 7}), true);
        assert_eq!(btree.search(IntKey{value: 8}), false);
        
        Ok(())
    }
}