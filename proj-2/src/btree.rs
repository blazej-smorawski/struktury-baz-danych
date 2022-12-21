use lru::LruCache;
use std::num::NonZeroUsize;
use std::{cell::RefCell, rc::Rc, vec};

use crate::pair::Pair;
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
    index_pages_max: u64,
    index_pages_free_list: Vec<u64>,
    data_pages_max: u64,
    data_pages_free_list: Vec<u64>,
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
            index_pages_max: 1,
            index_pages_free_list: vec![],
            data_pages_max: 0,
            data_pages_free_list: vec![],
            cache: LruCache::new(NonZeroUsize::new(4).unwrap()),
        };

        println!(
            "BTree:\n\t- degree: {}\n\t- index block size: {}\n\t- record size: {}\n",
            degree, block_size, record_size
        );

        btree
    }

    fn get_keys_count(page: &Page<BTreeRecord<K>>) -> usize {
        page.records.iter().filter(|x| x.key != K::invalid()).count()
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

    fn search(&mut self, key: K) -> Option<BTreeRecord<K>> {
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
                    return Some(*record.clone());
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
        None
    }

    fn get_next_index_lba(&mut self) -> u64 {
        if self.index_pages_free_list.is_empty() {
            let new_page = self.index_pages_max;
            self.index_pages_max += 1;
            self.index_pages_free_list.push(new_page);
        }

        // We take min and remove it from free list
        let min = *self.index_pages_free_list.iter().min().unwrap();
        let min_index = self.index_pages_free_list.iter().position(|x| *x == min).unwrap();
        self.index_pages_free_list.remove(min_index);

        min
    }

    fn get_next_data_lba(&mut self) -> u64 {
        if self.data_pages_free_list.is_empty() {
            let new_page = self.data_pages_max;
            self.data_pages_max += 1;
            self.data_pages_free_list.push(new_page);
        }

        // Just return min, because the insert is responsible for removing it
        *self.data_pages_free_list.iter().min().unwrap()
    }

    fn insert_record(&mut self, key: K, record: T, lba: u64) {
        let mut page = Page::<Pair<K, T>>::new(self.data_device.clone(), lba, u64::max_value());

        page.records.push(Box::new(Pair {
            key: key,
            value: record,
        }));
        page.dirty = true;

        let device_counted = self.data_device.clone();
        let device = device_counted.borrow();

        if (page.records.len() + 1) * Pair::<K, T>::get_size() as usize > device.block_size as usize {
            // Remove it from free list
            let lba_index = self.index_pages_free_list.iter().position(|x| *x == lba).unwrap();
            self.data_pages_free_list.remove(lba_index);
        }
    }

    fn delete_record(&mut self, key: K, lba: u64) {
        let mut page = Page::<Pair<K, T>>::new(self.data_device.clone(), lba, u64::max_value());

        let key_index = page.records.iter().position(|x| x.key == key).unwrap();
        page.records.remove(key_index);
        page.dirty = true;

        if self.data_pages_free_list.iter().find(|x| **x == lba).is_none() {
            self.data_pages_free_list.push(lba);
        }
    }

    fn get_record(&mut self, key: K, lba: u64) -> Option<T> {
        let page = Page::<Pair<K, T>>::new(self.data_device.clone(), lba, u64::max_value());

        match page.records.iter().find(|x| x.key == key) {
            Some(pair) => return Some(pair.value),
            None => return None,
        }
    }

    fn split_child(
        parent: &mut Page<BTreeRecord<K>>,
        child: &mut Page<BTreeRecord<K>>,
        new_child: &mut Page<BTreeRecord<K>>,
    ) {
        let centre_index = Self::get_keys_count(&child) / 2;
        let mut centre_record = child.records.remove(centre_index);

        for _ in centre_index..child.records.len() {
            new_child.records.push(child.records.remove(centre_index));
        }

        let child_record_index = parent
            .records
            .iter()
            .position(|x| x.child_lba == Some(child.lba))
            .expect("Tried to split `Page` that is not a child of `parent`");

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

    pub fn insert(&mut self, key: K, record: T) -> bool {
        {
            let root = self.get_page(0, u64::max_value());
            let mut root = root.borrow_mut();
            if Self::get_keys_count(&root) == (2 * self.degree - 1) as usize {
                let lba = self.index_pages_max;
                self.index_pages_max += 1;
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

                let data_lba = self.get_next_data_lba();

                self.insert_record(key, record, data_lba);

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

                let data_lba = self.get_next_data_lba();

                page.records.insert(
                    insert_index,
                    Box::new(BTreeRecord::<K> {
                        child_lba: None,
                        key: key,
                        data_lba: data_lba,
                    }),
                );

                self.insert_record(key, record, data_lba);

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

    fn can_borrow_from(&mut self, lba: u64, parent: u64) -> bool {
        let page_counted = self.get_page(lba, parent);
        let page = page_counted.borrow_mut();

        Self::get_keys_count(&page) >= self.degree as usize
    }

    pub fn find_min(&mut self, lba: u64, parent: u64) -> BTreeRecord<K> {
        let mut lba = lba;
        let mut parent = parent;
        loop {
            let page_counted = self.get_page(lba, parent);
            let page = page_counted.borrow();

            let first_record = page.records.first().unwrap();

            if let Some(next_lba) = first_record.child_lba {
                lba = next_lba;
                parent = page.lba;
            } else {
                return *first_record.clone();
            }
        }
    }

    pub fn find_max(&mut self, lba: u64, parent: u64) -> BTreeRecord<K> {
        let mut lba = lba;
        let mut parent = parent;
        loop {
            let page_counted = self.get_page(lba, parent);
            let page = page_counted.borrow();

            let last_record = page.records.last().unwrap();

            if let Some(next_lba) = last_record.child_lba {
                lba = next_lba;
                parent = page.lba;
            } else {
                return *last_record.clone();
            }
        }
    }

    fn join_children(&mut self, lba: u64, parent: u64, index: usize) -> (u64, u64) {
        let mut result_lba = 0;
        {
            let page_counted = self.get_page(lba, parent);
            let mut page = page_counted.borrow_mut();

            let left_child_lba = page.records[index].child_lba.unwrap();
            let left_child_counted = self.get_page(left_child_lba, page.lba);
            let mut left_child = left_child_counted.borrow_mut();

            let right_child_lba = page.records[index + 1].child_lba.unwrap();
            let right_child_counted = self.get_page(right_child_lba, page.lba);
            let mut right_child = right_child_counted.borrow_mut();

            // Pages prepared

            let moved_key = page.records[index].key;
            let moved_data = page.records[index].data_lba;
            page.records.remove(index);
            page.records[index].child_lba = Some(left_child_lba);

            // Jeśli łączymy liście, to musimy dodać nowy rekord
            // do którego włożymy rekord z `page`. Jeśli łączymy
            // nie-liście to nie musimy tego robić bo jest tam rekrod
            // w postaci ptr.*
            if left_child.records.last_mut().unwrap().key != K::invalid() {
                // Leaf
                left_child.records.push(Box::new(BTreeRecord::<K> {
                    child_lba: None,
                    key: K::invalid(),
                    data_lba: 0,
                }));
            }

            left_child.records.last_mut().unwrap().key = moved_key;
            left_child.records.last_mut().unwrap().data_lba = moved_data;
            left_child.records.append(&mut right_child.records);

            // Right child invalid
            self.

            left_child.dirty = true;
            right_child.dirty = true;
            page.dirty = true;

            result_lba = left_child.lba
        }

        self.fix_root(lba, parent, result_lba, lba)
    }

    fn fix_root(&mut self, lba: u64, parent: u64, new_root_lba: u64, new_root_parent: u64) -> (u64, u64) {
        let page_counted = self.get_page(lba, parent);
        let mut page = page_counted.borrow_mut();

        if page.lba == 0 && page.records.len() == 1 {
            // Empty root
            page.records.clear();

            let new_root_counted = self.get_page(new_root_lba, new_root_parent);
            let mut new_root = new_root_counted.borrow_mut();

            page.records.append(&mut new_root.records);

            page.dirty = true;
            new_root.dirty = true;

            return (page.lba, page.parent_lba);
        }

        (new_root_lba, new_root_parent)
    }

    fn get_child_at(&mut self, lba: u64, parent: u64, index: usize) -> (u64, u64) {
        let parent_counted = self.get_page(lba, parent);
        let parent = parent_counted.borrow();

        let child_lba = parent.records[index].child_lba.expect("Tried to enter leafs child!");
        let child_parent_lba = parent.lba;

        (child_lba, child_parent_lba)
    }

    fn can_borrow_from_child(&mut self, lba: u64, parent: u64, index: usize) -> bool {
        let parent_counted = self.get_page(lba, parent);
        let parent = parent_counted.borrow();

        let child_record = match parent.records.get(index) {
            Some(child) => child,
            None => return false,
        };

        self.can_borrow_from(
            child_record.child_lba.expect("Tried to borrow from leafs child!"),
            parent.lba,
        )
    }

    fn borrow_left(&mut self, lba: u64, parent: u64, index: usize) {
        let (brother_lba, brother_parent) = self.get_child_at(lba, parent, index - 1);
        let brother_counted = self.get_page(brother_lba, brother_parent);
        let mut brother = brother_counted.borrow_mut();

        let (target_lba, target_parent) = self.get_child_at(lba, parent, index);
        let target_counted = self.get_page(target_lba, target_parent);
        let mut target = target_counted.borrow_mut();

        let parent_counted = self.get_page(lba, parent);
        let mut parent = parent_counted.borrow_mut();

        std::mem::swap(
            &mut brother.records.last_mut().unwrap().key,
            &mut parent.records[index - 1].key,
        );
        target.records.insert(0, brother.records.pop().unwrap());

        if parent.records[index - 1].key == K::invalid() {
            std::mem::swap(
                &mut brother.records.last_mut().unwrap().key,
                &mut parent.records[index - 1].key,
            );
        }

        parent.dirty = true;
        brother.dirty = true;
        target.dirty = true;
    }

    fn borrow_right(&mut self, lba: u64, parent: u64, index: usize) {
        let (brother_lba, brother_parent) = self.get_child_at(lba, parent, index + 1);
        let brother_counted = self.get_page(brother_lba, brother_parent);
        let mut brother = brother_counted.borrow_mut();

        let (target_lba, target_parent) = self.get_child_at(lba, parent, index);
        let target_counted = self.get_page(target_lba, target_parent);
        let mut target = target_counted.borrow_mut();

        let parent_counted = self.get_page(lba, parent);
        let mut parent = parent_counted.borrow_mut();

        std::mem::swap(
            &mut brother.records.first_mut().unwrap().key,
            &mut parent.records[index].key,
        );
        target.records.push(brother.records.remove(0));

        if target.records.last().unwrap().child_lba != None {
            let last_record = target.records.len() - 1;
            let temp = target.records[last_record].key;
            target.records[last_record].key = target.records[last_record - 1].key;
            target.records[last_record - 1].key = temp;
        }

        parent.dirty = true;
        brother.dirty = true;
        target.dirty = true;
    }

    fn join_with_sibling(&mut self, lba: u64, parent: u64, index: usize) -> (u64, u64) {
        let sibling_index = index.checked_sub(1).unwrap_or(index);

        self.join_children(lba, parent, sibling_index)
    }

    pub fn prepare_for_remove(&mut self, lba: u64, parent: u64, index: usize) -> (u64, u64) {
        let (next_lba, next_parent) = self.get_child_at(lba, parent, index);
        // TODO: make it return lba, parent
        {
            let next_page_counted = self.get_page(next_lba, next_parent);
            let mut next_page = next_page_counted.borrow_mut();

            if Self::get_keys_count(&next_page) > (self.degree - 1) as usize {
                return (next_lba, next_parent);
            }
        }

        // ------ 3a ------
        if self.can_borrow_from_child(lba, parent, index.checked_sub(1).unwrap_or(usize::max_value())) {
            self.borrow_left(lba, parent, index);
            return (next_lba, next_parent);
        } else if self.can_borrow_from_child(lba, parent, index + 1) {
            self.borrow_right(lba, parent, index);
            return (next_lba, next_parent);
        }

        // ------ 3b ------
        self.join_with_sibling(lba, parent, index)
    }

    pub fn remove(&mut self, key: K) {
        self.remove_internal(key, 0, u64::max_value());
    }

    fn remove_internal(&mut self, key: K, lba: u64, parent: u64) {
        let mut index_option = None;
        {
            let page_counted = self.get_page(lba, parent);
            let page = page_counted.borrow_mut();
            index_option = page.records.iter().position(|x| x.key == key);
        }

        if let Some(index) = index_option {
            // Jest liściem
            {
                let page_counted = self.get_page(lba, parent);
                let mut page = page_counted.borrow_mut();

                // ------ 1 ------
                if page.records[index].child_lba == None {
                    page.records.remove(index);
                    page.dirty = true;
                    return;
                }

                // ------ 2 ------
                let left_child_lba = page.records[index].child_lba.unwrap();
                let right_child_lba = page.records[index + 1].child_lba.unwrap();

                // ------ 2a ------
                if self.can_borrow_from(left_child_lba, page.lba) {
                    let swap = self.find_max(left_child_lba, page.lba);
                    self.remove_internal(swap.key, left_child_lba, page.lba);

                    page.records[index].key = swap.key;
                    page.records[index].data_lba = swap.data_lba;

                    page.dirty = true;
                    return;
                } else if self.can_borrow_from(right_child_lba, page.lba) {
                    let swap = self.find_min(right_child_lba, page.lba);
                    self.remove_internal(swap.key, right_child_lba, page.lba);

                    page.records[index].key = swap.key;
                    page.records[index].data_lba = swap.data_lba;

                    page.dirty = true;
                    return;
                }
            }

            // ------ 2c ------
            let (next_lba, next_parent) = self.join_children(lba, parent, index);
            self.remove_internal(key, next_lba, next_parent);

            return;
        } else {
            let mut next_index = 0;

            // ------ 3 ------
            {
                let page_counted = self.get_page(lba, parent);
                let page = page_counted.borrow_mut();

                next_index = page
                    .records
                    .iter()
                    .position(|x| x.key == K::invalid() || x.key > key)
                    .expect("Could not find proper position for further search");
            }

            let (next_lba, next_parent) = self.prepare_for_remove(lba, parent, next_index);

            self.remove_internal(key, next_lba, next_parent);
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
        let mut data_device = BlockDevice::new("test_search_data.hex".to_string(), block_size, false).unwrap();
        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        assert_ne!(btree.search(IntKey { value: 20 }), None);
        assert_ne!(btree.search(IntKey { value: 10 }), None);
        assert_ne!(btree.search(IntKey { value: 7 }), None);
        assert_eq!(btree.search(IntKey { value: 8 }), None);

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

        let device = BlockDevice::new("test_insert_random.hex".to_string(), block_size, true).unwrap();
        let data_device = BlockDevice::new("test_insert_random_data.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let mut keys: Vec<i32> = (1..1000).collect();
        keys.shuffle(&mut thread_rng());
        for key in &keys {
            btree.insert(IntKey { value: *key }, IntRecord::from_string(key.to_string()).unwrap());
        }

        for key in &keys {
            assert_ne!(btree.search(IntKey { value: *key }), None);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_insert_increasing() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_insert_increasing.hex".to_string(), block_size, true).unwrap();
        let data_device = BlockDevice::new("test_insert_increasing_data.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let keys: Vec<i32> = (1..1000).collect();

        for key in &keys {
            btree.insert(IntKey { value: *key }, IntRecord::from_string(key.to_string()).unwrap());
        }

        for key in &keys {
            assert_ne!(btree.search(IntKey { value: *key }), None);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_insert_decreasing() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_insert_decreasing.hex".to_string(), block_size, true).unwrap();
        let data_device = BlockDevice::new("test_insert_decreasing_data.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let mut keys: Vec<i32> = (1..100).collect();
        keys.reverse();

        for key in &keys {
            btree.insert(IntKey { value: *key }, IntRecord::new());
        }

        for key in &keys {
            assert_ne!(btree.search(IntKey { value: *key }), None);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_insert_small_random_duplicates() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_insert_small_random_duplicates.hex".to_string(), block_size, true).unwrap();
        let data_device =
            BlockDevice::new("test_insert_small_random_duplicates.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let mut keys: Vec<i32> = (1..5).collect();
        keys.reverse();
        keys.shuffle(&mut thread_rng());

        for key in &keys {
            btree.insert(IntKey { value: *key }, IntRecord::new());
        }

        for key in &keys {
            btree.insert(IntKey { value: *key }, IntRecord::new());
        }

        for key in &keys {
            assert_ne!(btree.search(IntKey { value: *key }), None);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_remove_small() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_remove_leaf.hex".to_string(), block_size, true).unwrap();
        let data_device = BlockDevice::new("test_remove_leaf.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let keys: Vec<i32> = (1..5).collect();

        for key in &keys {
            btree.insert(IntKey { value: *key }, IntRecord::from_string(key.to_string()).unwrap());
        }

        btree.print();

        assert_ne!(btree.search(IntKey { value: 2 }), None);
        btree.remove(IntKey { value: 2 });
        assert_eq!(btree.search(IntKey { value: 2 }), None);
        btree.print();

        btree.remove(IntKey { value: 4 });
        assert_eq!(btree.search(IntKey { value: 4 }), None);
        btree.print();

        btree.remove(IntKey { value: 1 });
        assert_eq!(btree.search(IntKey { value: 1 }), None);
        btree.print();

        Ok(())
    }

    #[test]
    fn test_remove_medium() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_remove_medium.hex".to_string(), block_size, true).unwrap();
        let data_device = BlockDevice::new("test_remove_medium.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        //let mut keys: Vec<i32> = vec![5, 2, 1, 3, 4, 6, 10, 15, 20, 12,];
        //let keys_to_remove = vec![12, 4, 2, 3, 5, 10, 15, 1, 6];
        let mut keys: Vec<i32> = vec![5, 2, 1, 3, 4, 6, 10, 15, 20, 19, 18, 17, 12, 11, 9, 7, 8, 13, 14, 16];
        let keys_to_remove = vec![12, 9, 19, 2, 8, 7, 5, 10, 15, 1, 14, 20, 13, 6, 11, 18, 17, 16, 4, 3];
        //let mut keys_to_remove: Vec<i32> = keys.clone();
        //keys_to_remove.shuffle(&mut thread_rng());
        println!("{:?}", keys_to_remove);

        let mut keys_to_stay: Vec<i32> = keys.clone();

        for key in &keys {
            btree.insert(IntKey { value: *key }, IntRecord::from_string(key.to_string()).unwrap());
        }

        btree.print();

        for key in &keys_to_remove {
            println!("Removing {:?}", *key);

            assert_ne!(btree.search(IntKey { value: *key }), None);
            btree.remove(IntKey { value: *key });
            btree.print();

            assert_eq!(btree.search(IntKey { value: *key }), None);

            let index = keys_to_stay.iter().position(|x| *x == *key).unwrap();
            keys_to_stay.remove(index);

            for key in &keys_to_stay {
                assert_ne!(btree.search(IntKey { value: *key }), None);
            }
        }

        for key in &keys_to_stay {
            assert_ne!(btree.search(IntKey { value: *key }), None);
        }

        btree.print();

        Ok(())
    }

    #[test]
    fn test_remove_smoke() -> Result<(), std::io::Error> {
        let block_size = 21 * 4; // t = 2

        let device = BlockDevice::new("test_remove_smoke.hex".to_string(), block_size, true).unwrap();
        let data_device = BlockDevice::new("test_remove_smoke.hex".to_string(), block_size, true).unwrap();

        let mut btree = BTree::<IntKey, IntRecord>::new(device, data_device);

        let mut keys: Vec<i32> = (1..=1000).collect();
        keys.shuffle(&mut thread_rng());
        let mut keys_to_stay: Vec<i32> = keys.clone();

        let keys_to_remove: Vec<i32> = keys.choose_multiple(&mut rand::thread_rng(), 500).cloned().collect();

        for key in &keys {
            btree.insert(IntKey { value: *key }, IntRecord::from_string(key.to_string()).unwrap());
        }

        btree.print();

        for key in &keys_to_remove {
            assert_ne!(btree.search(IntKey { value: *key }), None);
            btree.remove(IntKey { value: *key });
            assert_eq!(btree.search(IntKey { value: *key }), None);

            let index = keys_to_stay.iter().position(|x| *x == *key).unwrap();
            keys_to_stay.remove(index);

            for key in &keys_to_stay {
                assert_ne!(btree.search(IntKey { value: *key }), None);
            }
        }

        btree.print();

        Ok(())
    }
}
