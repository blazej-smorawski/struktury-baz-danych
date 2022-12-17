use std::{cell::RefCell, rc::Rc, vec};
use std::num::NonZeroUsize;
use lru::LruCache;

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
    pub loaded_data: Vec<T>,
    pub degree: u64,
    pub pages_count: u64,
    cache: LruCache<u64 ,Rc<RefCell<Page<BTreeRecord<K>>>>>,
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
            loaded_data: vec![],
            degree: degree,
            pages_count: 1,
            cache: LruCache::new(NonZeroUsize::new(4).unwrap()),
        };

        println!(
            "BTree:\n\t- degree: {}\n\t- index block size: {}\n\t- record size: {}",
            degree, block_size, record_size
        );

        btree
    }

    fn get_keys_count(page: & Page<BTreeRecord<K>>) -> usize{
        page.records.iter().filter(|x| x.key != K::invalid()).count()
    }

    fn get_page(&mut self, lba: u64, parent: u64) -> Rc<RefCell<Page<BTreeRecord<K>>>> {
        self.cache.get_or_insert_mut(lba, || Rc::new(RefCell::new(Page::<BTreeRecord<K>>::new(self.index_device.clone(), lba, parent)))).clone()

    }

    fn search(&mut self, key: K) -> bool {
        let mut page_option = Some(self.get_page(0, u64::max_value()));

        while let Some(page) = page_option {
            let page = page.borrow();
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
                        page_option = Some(self.get_page(lba, page.lba))
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
        let centre_index = Self::get_keys_count(&child) / 2;
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

        let new_child_last_record = BTreeRecord::<K> {
            child_lba: centre_record.child_lba,
            key: K::invalid(),
            data_lba: 0,
        };
        if new_child_last_record != BTreeRecord::<K>::invalid() {
            child.records.push(Box::new(new_child_last_record));
        }

        centre_record.child_lba = Some(child.lba);
        parent.records[child_record_index].child_lba = Some(new_child.lba);
        parent.records.insert(child_record_index, centre_record);
        
        child.dirty = true;
        new_child.dirty = true;
        parent.dirty = true;
    }

    pub fn insert(&mut self, key: K) -> bool {
        {
            let root = self.get_page(0, u64::max_value());
            let mut root = root.borrow_mut();
            if Self::get_keys_count(&root) == (2 * self.degree - 1) as usize{
                let lba = self.pages_count;
                self.pages_count += 1;
                let working_page = self.get_page(lba, 0);
                let mut working_page = working_page.borrow_mut();

                /*
                * This record will land into root after swap
                */
                working_page.records.push(Box::new(BTreeRecord::<K> {
                    child_lba: Some(lba),
                    key: K::invalid(),
                    data_lba: 0,
                }));

                std::mem::swap(&mut working_page.records, &mut root.records);

                let next_index = self.get_next_index_lba();
                let new_page = self.get_page(next_index, 0);
                BTree::<K, T>::split_child(&mut root, &mut working_page, &mut new_page.borrow_mut());
            }
        }

        let mut next_lba = 0;
        let mut parent_lba = u64::max_value();
        loop {
            let page_counted = self.get_page(next_lba, parent_lba);
            let mut page = page_counted.borrow_mut();

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

                next_lba = page.records[next_search_index].child_lba.expect("Tried to enter leafs child!");
                parent_lba = page.lba;

                let mut parent = page;
                let page_counted = self.get_page(next_lba, parent_lba);
                let mut page = page_counted.borrow_mut();
    
                if Self::get_keys_count(&page) == (2 * self.degree - 1) as usize {
                    let next_index = self.get_next_index_lba();
                    let new_page = self.get_page(next_index, 0);
                    BTree::<K, T>::split_child(&mut parent, &mut page, &mut new_page.borrow_mut());

                    if parent.records[next_search_index].key < key {
                        next_lba = parent.records[next_search_index+1].child_lba.expect("Tried to enter leafs child!");
                    }
                }
            }
        }

        true
    }

    pub fn print(&mut self) {
        let mut tree = Vec::<Vec<Rc<RefCell<Page<BTreeRecord<K>>>>>>::new();
        let root = self.get_page(0, u64::max_value());

        tree.push(vec![root]);
        let mut level = &tree[0];
        loop {
            let mut next_level = vec![];
            for page in level {
                let page = page.borrow();
                for record in &page.records {
                    if let Some(child) = record.child_lba {
                        let new_page = self.get_page(child, page.lba);
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

        let width = 34;
        let max_width = width * tree.last().unwrap().len();

        for level in tree {
            let pages_count = level.len();
            for page in level {
                let page = page.borrow();
                let mut count = 0;

                let mut page_format = format!("{:>3}>[", page.lba);
                for record in &page.records {
                    let record_format = format!("{:^5},", format!("{}", record));
                    page_format = format!("{}{}",page_format, record_format);
                    count += 1;
                }
                while count<self.degree*2 {
                    let record_format = format!("{},", BTreeRecord::<K>::invalid());
                    page_format = format!("{}{}",page_format,record_format);
                    count += 1;
                }
                let row_width = max_width/pages_count;
                page_format = format!("{}]",page_format);
                print!("{}", format!("{:^row_width$}", page_format));
            }
            print!("\n\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{seq::SliceRandom, thread_rng};

    use crate::{btree_key::IntKey, record::IntRecord};

    use super::*;

    #[test]
    fn test_search() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        {
            let device = BlockDevice::new("test_search.hex".to_string(), block_size, true).unwrap();
            let device = Rc::new(RefCell::new(device));
            let mut root_page = Page::<BTreeRecord<IntKey>>::new(device.clone(), 0, 0);
            let mut child1_page = Page::<BTreeRecord<IntKey>>::new(device.clone(), 1, 0);
            let mut child2_page = Page::<BTreeRecord<IntKey>>::new(device.clone(), 2, 0);
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
        let mut root_page = Page::<BTreeRecord<IntKey>>::new(device.clone(), 0, 0);
        let mut child_page = Page::<BTreeRecord<IntKey>>::new(device.clone(), 1, 0);

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
    fn test_split_with_children() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_split_with_children.hex".to_string(), block_size, true).unwrap();
        let device = Rc::new(RefCell::new(device));
        let mut root_page = Page::<BTreeRecord<IntKey>>::new(device.clone(), 0, 0);
        let mut child_page = Page::<BTreeRecord<IntKey>>::new(device.clone(), 1, 0);

        let record1 = BTreeRecord::<IntKey> {
            child_lba: Some(1),
            key: IntKey::invalid(),
            data_lba: 0,
        };
        root_page.records.push(Box::new(record1));

        let record1 = BTreeRecord::<IntKey> {
            child_lba: Some(1),
            key: IntKey { value: 11 },
            data_lba: 0,
        };
        let record2 = BTreeRecord::<IntKey> {
            child_lba: Some(2),
            key: IntKey { value: 13 },
            data_lba: 0,
        };
        let record3 = BTreeRecord::<IntKey> {
            child_lba: Some(3),
            key: IntKey { value: 16 },
            data_lba: 0,
        };
        let record4 = BTreeRecord::<IntKey> {
            child_lba: Some(4),
            key: IntKey::invalid(),
            data_lba: 0,
        };
        child_page.records.push(Box::new(record1));
        child_page.records.push(Box::new(record2));
        child_page.records.push(Box::new(record3));
        child_page.records.push(Box::new(record4));

        let mut new_page = Page::<BTreeRecord<IntKey>>::empty(&device.clone(), 2, 0);
        BTree::<IntKey, IntRecord>::split_child(&mut root_page, &mut child_page, &mut new_page);

        // assert_eq!(child_page.records, vec![record1, BTreeRecord::<IntKey> {
        //     child_lba: Some(3),
        //     key: IntKey { value: 16 },
        //     data_lba: 0,
        // }]);

        Ok(())
    }

    #[test]
    fn test_insert_random() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_insert.hex".to_string(), block_size, true).unwrap();
        let data_device = BlockDevice::new("test_insert_data.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let mut keys: Vec<i32> = (1..100).collect();
        keys.shuffle(&mut thread_rng());
        for key in &keys {
            btree.insert(IntKey{value: *key});
        }

        for key in &keys {
            assert_eq!(btree.search(IntKey{value: *key}), true);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_insert_increasing() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_insert.hex".to_string(), block_size, true).unwrap();
        let data_device = BlockDevice::new("test_insert_data.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let keys: Vec<i32> = (1..100).collect();

        for key in &keys {
            btree.insert(IntKey{value: *key});
        }

        for key in &keys {
            assert_eq!(btree.search(IntKey{value: *key}), true);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_insert_decreasing() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_insert.hex".to_string(), block_size, true).unwrap();
        let data_device = BlockDevice::new("test_insert_data.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let mut keys: Vec<i32> = (1..100).collect();
        keys.reverse();

        for key in &keys {
            btree.insert(IntKey{value: *key});
        }

        for key in &keys {
            assert_eq!(btree.search(IntKey{value: *key}), true);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_insert_small_random() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_insert.hex".to_string(), block_size, true).unwrap();
        let data_device = BlockDevice::new("test_insert_data.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let mut keys: Vec<i32> = (1..20).collect();
        keys.reverse();
        keys.shuffle(&mut thread_rng());

        for key in &keys {
            btree.insert(IntKey{value: *key});
        }

        for key in &keys {
            assert_eq!(btree.search(IntKey{value: *key}), true);
        }

        btree.print();

        Ok(())
    }
}
