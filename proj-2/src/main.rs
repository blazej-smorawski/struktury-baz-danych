pub mod btree;
pub mod btree_key;
pub mod btree_record;
pub mod bytes;
pub mod device;
pub mod page;
pub mod pair;
pub mod record;
use std::{io::{self, BufRead}, rc::Rc, cell::RefCell};

use crate::{btree::BTree, btree_key::IntKey, device::BlockDevice, record::{IntRecord, Record}};
use clap::Parser;
use colored::Colorize;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = true)]
    truncate: bool,

    #[arg(short, long, default_value_t = 84)]
    index_block_size: u64,

    #[arg(short, long, default_value_t = 256)]
    data_block_size: u64,
}

fn main() {
    let index_device = Rc::new(RefCell::new(BlockDevice::new("index.hex".to_string(), 84, true).expect("Could not create index device")));
    let data_device =Rc::new(RefCell::new( BlockDevice::new("data.hex".to_string(), 256, true).expect("Could not create data device")));

    {
        let mut b_tree = BTree::<IntKey, IntRecord>::new(index_device.clone(), data_device.clone());

        println!("Available operations: insert, remove, search, print, print data, print stats");
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        loop {
            let mut input = String::new();
            let read_line_result = handle.read_line(&mut input);
            match read_line_result {
                Ok(_) => (),
                Err(_) => { break }
            }

            if input.is_empty() {
                break;
            }

            match input.as_str().trim()  {
                "insert" => {
                    let mut insert_input = String::new();
                    handle.read_line(&mut insert_input).unwrap();
                    let substrings: Vec<&str> = insert_input.split(':').collect();
                    let key_value = match substrings[0].parse() {
                        Ok(key) => key,
                        Err(_) =>  {
                            println!("{}", format!("Could not read the key!").red());
                            continue;
                        }
                    };
                    let key = IntKey{ value: key_value};
                    let record = match IntRecord::from_string(substrings[1].to_string()) {
                        Ok(record) => record,
                        Err(_) =>  {
                            println!("{}", format!("Could not read the record!").red());
                            continue;
                        }
                    };
                    b_tree.insert(key, record);
                },
                "remove" => {
                    let mut insert_input = String::new();
                    handle.read_line(&mut insert_input).unwrap();
                    let key = IntKey{ value: insert_input.trim().parse().unwrap()};

                    b_tree.remove(key);
                },
                "search" => {
                    let mut insert_input = String::new();
                    handle.read_line(&mut insert_input).unwrap();
                    let key = IntKey{ value: insert_input.parse().unwrap()};
                    match b_tree.search(key){
                        Some(_) => println!("{}", format!("Found").green()),
                        None => println!("{}", format!("Not found").yellow())
                    };
                },
                "print" => {
                    b_tree.print();
                },
                "print data" => {
                    b_tree.print_data();
                },
                "print stats" => {
                    let index_device = index_device.borrow();
                    let data_device = data_device.borrow();
                    println!("{}", format!("Index:\tReads->{}\tWrites->{}\tSize->{}", index_device.reads, index_device.writes, index_device.get_size()).blue());
                    println!("{}", format!("Data:\tReads->{}\tWrites->{}\tSize->{}", data_device.reads, data_device.writes, data_device.get_size()).blue());
                },
                _ => {
                    println!("{}", format!("Unknown operation!").red())
                }
            }
        }
    }

    let index_device = index_device.borrow();
    let data_device = data_device.borrow();
    println!("{}", format!("Index:\tReads->{}\tWrites->{}\tSize->{}", index_device.reads, index_device.writes, index_device.get_size()).blue());
    println!("{}", format!("Data:\tReads->{}\tWrites->{}\tSize->{}", data_device.reads, data_device.writes, data_device.get_size()).blue());
}
