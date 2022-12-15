pub mod device;
pub mod record;
pub mod btree;
pub mod btree_key;
pub mod btree_record;
pub mod btree_page;

use std::vec;

use crate::{device::BlockDevice, btree::{BTree}, record::IntRecord, btree_key::IntKey};


fn main() {
    let index_device = BlockDevice::new("index.hex".to_string(), 256, true).expect("Could not create index device");
    let data_device = BlockDevice::new("index.hex".to_string(), 256, true).expect("Could not create data device");
    let b_tree = BTree::<IntKey, IntRecord>::new(index_device, data_device);

    println!("Hello, world!");
}
