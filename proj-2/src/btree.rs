use lru::LruCache;
use std::num::NonZeroUsize;
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
    pub loaded_data: Vec<T>,
    pub degree: u64,
    pub pages_count: u64,
    cache: LruCache<u64, Rc<RefCell<Page<BTreeRecord<K>>>>>,
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

    fn get_keys_count(page: &Page<BTreeRecord<K>>) -> usize {
        page.records
            .iter()
            .filter(|x| x.key != K::invalid())
            .count()
    }

    fn get_page(&mut self, lba: u64, parent: u64) -> Rc<RefCell<Page<BTreeRecord<K>>>> {
        self.cache
            .get_or_insert_mut(lba, || {
                Rc::new(RefCell::new(Page::<BTreeRecord<K>>::new(
                    self.index_device.clone(),
                    lba,
                    parent,
                )))
            })
            .clone()
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
                    Some(lba) => page_option = Some(self.get_page(lba, page.lba)),
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
            if Self::get_keys_count(&root) == (2 * self.degree - 1) as usize {
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
                BTree::<K, T>::split_child(
                    &mut root,
                    &mut working_page,
                    &mut new_page.borrow_mut(),
                );
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
                let is_present = page.records.iter().find(|x| x.key == key);

                if is_present.is_some() {
                    return true;
                }

                let insert_index = page
                    .records
                    .iter()
                    .position(|x| x.key == K::invalid() || x.key >= key)
                    .unwrap_or(page.records.len());

                page.records.insert(
                    insert_index,
                    Box::new(BTreeRecord::<K> {
                        child_lba: None,
                        key: key,
                        data_lba: 0,
                    }),
                );
                page.dirty = true;
                break;
            } else {
                /*
                 * Non-leaf page
                 */
                let is_present = page.records.iter().find(|x| x.key == key);

                if is_present.is_some() {
                    return true;
                }

                let next_search_index = page
                    .records
                    .iter()
                    .position(|x| x.key == K::invalid() || x.key > key)
                    .expect("Could not find proper position for further search");

                next_lba = page.records[next_search_index]
                    .child_lba
                    .expect("Tried to enter leafs child!");
                parent_lba = page.lba;

                let mut parent = page;
                let page_counted = self.get_page(next_lba, parent_lba);
                let mut page = page_counted.borrow_mut();

                if Self::get_keys_count(&page) == (2 * self.degree - 1) as usize {
                    let next_index = self.get_next_index_lba();
                    let new_page = self.get_page(next_index, 0);
                    BTree::<K, T>::split_child(&mut parent, &mut page, &mut new_page.borrow_mut());

                    if parent.records[next_search_index].key < key {
                        next_lba = parent.records[next_search_index + 1]
                            .child_lba
                            .expect("Tried to enter leafs child!");
                    }
                }
            }
        }

        true
    }

    pub fn remove(&mut self, key: K) {
        let root_counted = self.get_page(0, u64::max_value());
        let mut page = root_counted.borrow_mut();
        self.remove_internal(key, &mut page);
    }

    pub fn remove_internal(&mut self, key: K, page: &mut Page<BTreeRecord<K>>) {
        // ------ 1 ------
        let index_option = page.records.iter().position(|x| x.key == key);

        // Jest w tym węźle
        if let Some(index) = index_option {
            // Jest liściem
            if page.records[index].child_lba == None {
                page.records.remove(index);
                page.dirty = true;
                return;
            } else {
                let left_child_lba = page.records[index].child_lba.unwrap();
                let left_child_counted = self.get_page(left_child_lba, page.lba);
                let mut left_child = left_child_counted.borrow_mut();

                if Self::get_keys_count(&left_child) >= self.degree as usize {
                    // ------ 2a ------
                    let last_key_index = left_child
                        .records
                        .iter()
                        .filter(|x| x.key != K::invalid())
                        .count()
                        - 1;
                    let predecessor_key = left_child.records[last_key_index].key;
                    let predecessor_data = left_child.records[last_key_index].data_lba;

                    self.remove_internal(predecessor_key, &mut left_child);

                    page.records[index].key = predecessor_key;
                    page.records[index].data_lba = predecessor_data;
                    page.dirty = true;
                    return;
                } else {
                    let right_child_lba = page.records[index + 1].child_lba.unwrap();
                    let right_child_counted = self.get_page(right_child_lba, page.lba);
                    let mut right_child = right_child_counted.borrow_mut();

                    if Self::get_keys_count(&right_child) >= self.degree as usize {
                        // ------ 2b ------
                        let successor_key = right_child.records.first().unwrap().key;
                        let successor_data = right_child.records.first().unwrap().data_lba;

                        self.remove_internal(successor_key, &mut right_child);

                        page.records[index].key = successor_key;
                        page.records[index].data_lba = successor_data;
                        page.dirty = true;
                        return;
                    } else {
                        // ------ 2c ------
                        let moved_key = page.records[index].key;
                        let moved_data = page.records[index].data_lba;
                        page.records.remove(index);
                        page.records[index].child_lba = Some(left_child_lba);

                        if left_child.records.last_mut().unwrap().key != K::invalid() {
                            // Leaf
                            left_child.records.push(Box::new(BTreeRecord::<K> {
                                child_lba: None,
                                key: K::invalid(),
                                data_lba: 0,
                            }));
                        }

                        let left_child_last_index = left_child.records.len() - 1;
                        left_child.records[left_child_last_index].key = moved_key;
                        left_child.records[left_child_last_index].data_lba = moved_data;
                        left_child.records.append(&mut right_child.records);

                        self.remove_internal(key, &mut left_child);

                        if page.lba == 0 && page.records.len() == 1 {
                            // Empty root
                            page.records.clear();
                            page.records.append(&mut left_child.records);
                        }

                        page.dirty = true;
                        return;
                    }
                }
            }
        } else {
            // ------ 3 ------
            let next_search_index = page
                .records
                .iter()
                .position(|x| x.key == K::invalid() || x.key > key)
                .expect("Could not find proper position for further search");

            let next_lba = page.records[next_search_index]
                .child_lba
                .expect("Tried to enter leafs child!");
            let next_parent_lba = page.lba;

            let next_page_counted = self.get_page(next_lba, next_parent_lba);
            let mut next_page = next_page_counted.borrow_mut();

            if Self::get_keys_count(&next_page) == (self.degree - 1) as usize {
                let left_brother = page.records.get(
                    next_search_index
                        .checked_sub(1)
                        .unwrap_or(usize::max_value()),
                );
                let right_brother = page.records.get(next_search_index + 1);
                let brothers = vec![left_brother, right_brother];

                let brother_option = brothers.iter().flatten().find(|x| {
                    let brother_counted = self.get_page(x.child_lba.unwrap(), page.lba);
                    let brother_page = brother_counted.borrow();
                    Self::get_keys_count(&brother_page) >= self.degree as usize
                });

                if let Some(brother) = brother_option {
                    // ------ 3a ------
                    let brother_counted = self.get_page(brother.child_lba.unwrap(), page.lba);
                    let mut brother_page = brother_counted.borrow_mut();

                    if page
                        .records
                        .iter()
                        .position(|x| x.key == brother.key)
                        .unwrap()
                        < next_search_index
                    {
                        // Lewy brat
                        let moved_index = brother_page.records.len() - 1;
                        let mut moved_to_right = brother_page.records.remove(moved_index);
                        std::mem::swap(&mut moved_to_right.key, &mut page.records[next_search_index - 1].key);
                        //moved_to_right.data_lba = page.records[next_search_index - 1].data_lba;
                        next_page.records.insert(0, moved_to_right);
                        
                    } else {
                        // Prawy brat
                        if next_page.records.last_mut().unwrap().key != K::invalid() {
                            // Leaf
                            next_page.records.push(Box::new(BTreeRecord::<K> {
                                child_lba: None,
                                key: K::invalid(),
                                data_lba: 0,
                            }));
                        }

                        next_page.records.last_mut().unwrap().key =
                            page.records[next_search_index].key;
                        next_page.records.last_mut().unwrap().data_lba =
                            page.records[next_search_index].data_lba;

                        let moved_index = 0;
                        let mut moved_to_left = brother_page.records.remove(moved_index);
                        page.records[next_search_index].key = moved_to_left.key;
                        page.records[next_search_index].data_lba = moved_to_left.data_lba;

                        moved_to_left.key = K::invalid();
                        moved_to_left.data_lba = 0;
                        if moved_to_left.child_lba != None {
                            next_page.records.push(moved_to_left);
                        }
                    }

                    page.dirty = true;
                    next_page.dirty = true;
                    brother_page.dirty = true;
                } else {
                    // ------ 3b ------
                    let brother = brothers.iter().flatten().next().unwrap();
                    let brother_counted = self.get_page(brother.child_lba.unwrap(), page.lba);
                    let mut brother_page = brother_counted.borrow_mut();

                    if page
                        .records
                        .iter()
                        .position(|x| x.key == brother.key)
                        .unwrap()
                        < next_search_index
                    {
                        if brother_page.records.last_mut().unwrap().key != K::invalid() {
                            // Leaf
                            brother_page.records.push(Box::new(BTreeRecord::<K> {
                                child_lba: None,
                                key: K::invalid(),
                                data_lba: 0,
                            }));
                        }
                        brother_page.records.last_mut().unwrap().key =
                            page.records[next_search_index - 1].key;
                        brother_page.records.last_mut().unwrap().data_lba =
                            page.records[next_search_index - 1].data_lba;

                        let mut index = 0;
                        while !brother_page.records.is_empty() {
                            let record = brother_page.records.remove(0);
                            next_page.records.insert(index, record);
                            index += 1;
                        }

                        page.records.remove(next_search_index - 1);
                    } else {
                        if next_page.records.last_mut().unwrap().key != K::invalid() {
                            // Leaf
                            next_page.records.push(Box::new(BTreeRecord::<K> {
                                child_lba: None,
                                key: K::invalid(),
                                data_lba: 0,
                            }));
                        }
                        next_page.records.last_mut().unwrap().key =
                            page.records[next_search_index].key;
                        next_page.records.last_mut().unwrap().data_lba =
                            page.records[next_search_index].data_lba;

                        while !brother_page.records.is_empty() {
                            let record = brother_page.records.remove(0);
                            next_page.records.push(record);
                        }

                        page.records[next_search_index + 1].child_lba =
                            page.records[next_search_index].child_lba;
                        page.records.remove(next_search_index);
                    }

                    page.dirty = true;
                    next_page.dirty = true;
                    brother_page.dirty = true;
                }
            }

            self.remove_internal(key, &mut next_page);

            if page.lba == 0 && page.records.len() == 1 {
                // Empty root
                page.records.clear();
                page.records.append(&mut next_page.records);
                page.dirty = true;
                next_page.dirty = true;
            }
        }
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
                    page_format = format!("{}{}", page_format, record_format);
                    count += 1;
                }
                while count < self.degree * 2 {
                    let record_format = format!("{},", BTreeRecord::<K>::invalid());
                    page_format = format!("{}{}", page_format, record_format);
                    count += 1;
                }
                let row_width = max_width / pages_count;
                page_format = format!("{}]", page_format);
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

        let device =
            BlockDevice::new("test_split_with_children.hex".to_string(), block_size, true).unwrap();
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
        let data_device =
            BlockDevice::new("test_insert_data.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let mut keys: Vec<i32> = (1..1000).collect();
        keys.shuffle(&mut thread_rng());
        for key in &keys {
            btree.insert(IntKey { value: *key });
        }

        for key in &keys {
            assert_eq!(btree.search(IntKey { value: *key }), true);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_insert_increasing() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_insert.hex".to_string(), block_size, true).unwrap();
        let data_device =
            BlockDevice::new("test_insert_data.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let keys: Vec<i32> = (1..1000).collect();

        for key in &keys {
            btree.insert(IntKey { value: *key });
        }

        for key in &keys {
            assert_eq!(btree.search(IntKey { value: *key }), true);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_insert_decreasing() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_insert.hex".to_string(), block_size, true).unwrap();
        let data_device =
            BlockDevice::new("test_insert_data.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let mut keys: Vec<i32> = (1..100).collect();
        keys.reverse();

        for key in &keys {
            btree.insert(IntKey { value: *key });
        }

        for key in &keys {
            assert_eq!(btree.search(IntKey { value: *key }), true);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_insert_small_random() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_insert.hex".to_string(), block_size, true).unwrap();
        let data_device =
            BlockDevice::new("test_insert_data.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let mut keys: Vec<i32> = (1..5).collect();
        keys.reverse();
        keys.shuffle(&mut thread_rng());

        for key in &keys {
            btree.insert(IntKey { value: *key });
        }

        for key in &keys {
            btree.insert(IntKey { value: *key });
        }

        for key in &keys {
            assert_eq!(btree.search(IntKey { value: *key }), true);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_remove_small() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device =
            BlockDevice::new("test_remove_leaf.hex".to_string(), block_size, true).unwrap();
        let data_device =
            BlockDevice::new("test_remove_leaf.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let keys: Vec<i32> = (1..5).collect();

        for key in &keys {
            btree.insert(IntKey { value: *key });
        }

        btree.print();

        assert_eq!(btree.search(IntKey { value: 2 }), true);
        btree.remove(IntKey { value: 2 });
        assert_eq!(btree.search(IntKey { value: 2 }), false);
        btree.print();

        btree.remove(IntKey { value: 4 });
        assert_eq!(btree.search(IntKey { value: 4 }), false);
        btree.print();

        btree.remove(IntKey { value: 1 });
        assert_eq!(btree.search(IntKey { value: 1 }), false);
        btree.print();

        btree.print();

        Ok(())
    }

    #[test]
    fn test_remove_medium() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device =
            BlockDevice::new("test_remove_medium.hex".to_string(), block_size, true).unwrap();
        let data_device =
            BlockDevice::new("test_remove_medium.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let mut keys: Vec<i32> = vec![
            5, 2, 1, 3, 4, 6, 10, 15, 20, 19, 18, 17, 12, 11, 9, 7, 8, 13, 14, 16,
        ];

        let keys_to_remove = vec![
            12, 9, 19, 2, 8, 7, 5, 10, 15, 1, 14, 20, 13, 6, 11, 18, 17, 16, 4, 3,
        ];
        //let mut keys_to_remove: Vec<i32> = keys.clone();
        //keys_to_remove.shuffle(&mut thread_rng());
        println!("{:?}", keys_to_remove);

        let keys_to_stay: Vec<i32> = (1..=20).filter(|x| !keys_to_remove.contains(x)).collect();

        for key in &keys {
            btree.insert(IntKey { value: *key });
        }

        btree.print();

        for key in &keys_to_remove {
            println!("Removing {:?}", *key);
            assert_eq!(btree.search(IntKey { value: *key }), true);
            btree.remove(IntKey { value: *key });
            assert_eq!(btree.search(IntKey { value: *key }), false);
            btree.print();
        }

        for key in &keys_to_stay {
            assert_eq!(btree.search(IntKey { value: *key }), true);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_remove_smoke() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device =
            BlockDevice::new("test_remove_smoke.hex".to_string(), block_size, true).unwrap();
        let data_device =
            BlockDevice::new("test_remove_smoke.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let keys: Vec<i32> = (1..=1000).collect();

        let keys_to_remove: Vec<i32> = keys
            .choose_multiple(&mut rand::thread_rng(), 500)
            .cloned()
            .collect();
        let keys_to_stay: Vec<i32> = (1..=20).filter(|x| !keys_to_remove.contains(x)).collect();

        for key in &keys {
            btree.insert(IntKey { value: *key });
        }

        btree.print();

        for key in &keys_to_remove {
            assert_eq!(btree.search(IntKey { value: *key }), true);
            btree.remove(IntKey { value: *key });
            assert_eq!(btree.search(IntKey { value: *key }), false);
        }

        for key in &keys_to_stay {
            assert_eq!(btree.search(IntKey { value: *key }), true);
        }

        btree.print();

        Ok(())
    }
}
