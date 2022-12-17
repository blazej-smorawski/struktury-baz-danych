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
    pub helper_page: Option<Page<BTreeRecord<K>>>,
    pub loaded_data: Vec<T>,
    pub degree: u64,
    pub pages_count: u64,
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
            index_root: Page::new(&index_device.clone(), 0, u64::max_value()),
            working_page: None,
            helper_page: None,
            loaded_data: vec![],
            degree: degree,
            pages_count: 1,
        };

        println!(
            "BTree:\n\t- degree: {}\n\t- index block size: {}\n\t- record size: {}",
            degree, block_size, record_size
        );

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
                break;
            }
        }
        /*
         * We did not find anything
         */
        false
    }

    fn get_next_index_lba(&mut self) -> u64 {
        let ret = self.pages_count;
        self.pages_count += 1;
        ret
    }

    fn split_child(
        mut parent: &mut Page<BTreeRecord<K>>,
        mut child: &mut Page<BTreeRecord<K>>,
        mut new_child: &mut Page<BTreeRecord<K>>,
    ) {
        let centre_index = child.records.len() / 2;
        let mut centre_record = child.records.remove(centre_index);

        for _ in centre_index..child.records.len() {
            new_child.records.push(child.records.remove(centre_index));
        }

        let child_record_index_option = parent
            .records
            .iter()
            .position(|x| x.child_lba == Some(child.lba));
        let child_record_index = match child_record_index_option {
            Some(index) => index,
            None => panic!("Tried to split `Page` that is not a child of `parent`"),
        };

        let new_page_last_record = BTreeRecord::<K> {
            child_lba: centre_record.child_lba,
            key: K::invalid(),
            data_lba: 0,
        };
        new_child.records.push(Box::new(new_page_last_record));
        centre_record.child_lba = Some(child.lba);
        parent.records[child_record_index].child_lba = Some(new_child.lba);
        parent.records.insert(child_record_index, centre_record);
    }

    pub fn insert(&mut self, key: K) -> bool {
        if self.index_root.records.iter().filter(|x| x.key != K::invalid()).count() == (2 * self.degree - 1) as usize{
            let lba = self.get_next_index_lba();
            let mut working_page =
                Page::<BTreeRecord<K>>::empty(&self.index_device.clone(), lba, 0);

            /*
             * This record will land into root after swap
             */
            working_page.records.push(Box::new(BTreeRecord::<K> {
                child_lba: Some(lba),
                key: K::invalid(),
                data_lba: 0,
            }));

            std::mem::swap(&mut working_page.records, &mut self.index_root.records);

            let mut new_page = Page::<BTreeRecord<K>>::empty(
                &self.index_device.clone(),
                self.get_next_index_lba(),
                0,
            );
            BTree::<K, T>::split_child(&mut self.index_root, &mut working_page, &mut new_page);
            self.working_page = Some(working_page);
        }

        let mut page = &mut self.index_root;
        loop {
            if page.records.is_empty() {
                page.records.push(Box::new(BTreeRecord::<K> {
                    child_lba: None,
                    key: key,
                    data_lba: 0,
                }));
                page.dirty = true;
                break;
            } else if page.records[0].child_lba == None {
                let insert_index = page
                    .records
                    .iter()
                    .position(|x| x.key == K::invalid() || x.key >= key)
                    .unwrap_or(page.records.len());

                page.records.insert(insert_index, Box::new(BTreeRecord::<K> {
                    child_lba: None,
                    key: key,
                    data_lba: 0,
                }));
                page.dirty = true;
                break;
            } else {
                /*
                 * Non-leaf page
                 */
                let next_search_index = page
                    .records
                    .iter()
                    .position(|x| x.key == K::invalid() || x.key > key)
                    .expect("Could not find proper position for further search");

                let next_lba = page.records[next_search_index].child_lba.expect("Tried to enter leafs child!");

                self.working_page = Some(Page::<BTreeRecord<K>>::new(
                    &self.index_device.clone(),
                    next_lba,
                    0,
                ));
                page = self.working_page.as_mut().unwrap();
            }
        }

        true
    }

    pub fn print(&mut self) {
        let mut tree = Vec::<Vec::<Page::<BTreeRecord<K>>>>::new();
        let root = Page::<BTreeRecord<K>>::new(&self.index_device, 0, u64::max_value());

        tree.push(vec![root]);
        let mut level = &tree[0];
        loop {
            let mut next_level = vec![];
            for page in level {
                for record in &page.records {
                    if let Some(child) = record.child_lba {
                        let new_page = Page::<BTreeRecord<K>>::new(&self.index_device, child, page.lba);
                        next_level.push(new_page);
                    }
                }
            }

            if next_level.is_empty() {
                break;
            } else {
                tree.push(next_level);
                level = &tree.last().unwrap();
            }
        }

        for level in tree {
            for page in level {
                let mut count = 0;
                print!("[");
                for record in &page.records {
                    print!("{}", record);
                    count += 1;
                }
                while count<self.degree*2 {
                    print!("{}", BTreeRecord::<K>::invalid());
                    count += 1;
                }
                print!("]");
            }
            print!("\n\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{btree_key::IntKey, record::IntRecord};

    use super::*;

    #[test]
    fn test_search() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

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
        let mut data_device =
            BlockDevice::new("test_search_data.hex".to_string(), block_size, false).unwrap();
        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        assert_eq!(btree.search(IntKey { value: 20 }), true);
        assert_eq!(btree.search(IntKey { value: 10 }), true);
        assert_eq!(btree.search(IntKey { value: 7 }), true);
        assert_eq!(btree.search(IntKey { value: 8 }), false);

        Ok(())
    }

    #[test]
    fn test_split() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_split.hex".to_string(), block_size, true).unwrap();
        let device = Rc::new(RefCell::new(device));
        let mut root_page = Page::<BTreeRecord<IntKey>>::new(&device.clone(), 0, 0);
        let mut child_page = Page::<BTreeRecord<IntKey>>::new(&device.clone(), 1, 0);

        let record1 = BTreeRecord::<IntKey> {
            child_lba: Some(1),
            key: IntKey::invalid(),
            data_lba: 0,
        };
        root_page.records.push(Box::new(record1));

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
        let record3 = BTreeRecord::<IntKey> {
            child_lba: None,
            key: IntKey { value: 10 },
            data_lba: 0,
        };
        child_page.records.push(Box::new(record1));
        child_page.records.push(Box::new(record2));
        child_page.records.push(Box::new(record3));

        let mut new_page = Page::<BTreeRecord<IntKey>>::empty(&device.clone(), 2, 0);
        BTree::<IntKey, IntRecord>::split_child(&mut root_page, &mut child_page, &mut new_page);

        Ok(())
    }

    #[test]
    fn test_insert() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_insert.hex".to_string(), block_size, true).unwrap();
        let data_device = BlockDevice::new("test_insert_data.hex".to_string(), block_size, true).unwrap();

        {
            let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

            btree.insert(IntKey{value: 10});
            assert_eq!(btree.search(IntKey{value: 10}), true);
            assert_eq!(btree.search(IntKey{value: 11}), false);

            btree.insert(IntKey{value: 11});
            btree.insert(IntKey{value: 12});
            btree.insert(IntKey{value: 13});
            // assert_eq!(btree.search(IntKey{value: 10}), true);
            // assert_eq!(btree.search(IntKey{value: 11}), true);
            // assert_eq!(btree.search(IntKey{value: 12}), true);
            // assert_eq!(btree.search(IntKey{value: 13}), true);
            // assert_eq!(btree.search(IntKey{value: 14}), false);
            btree.insert(IntKey{value: 14});
        }

        let device = BlockDevice::new("test_insert.hex".to_string(), block_size, false).unwrap();
        let data_device = BlockDevice::new("test_insert_data.hex".to_string(), block_size, true).unwrap();
        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);
        btree.print();

        Ok(())
    }
}
