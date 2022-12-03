pub mod device;
pub mod record;
pub mod tape;
use std::io::BufRead;
use std::{env, io};

use crate::record::{IntRecord, Record};

use crate::device::BlockDevice;

use crate::tape::Tape;

fn help() {
    println!(
        "usage:
    -s
    -f <path>
    -r <count>"
    );
}

fn main() {
    //run().expect("Error while running:");
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => help(),
        _ => (),
    }

    let mut device: BlockDevice;
    let mut tape: Tape<IntRecord>;
    let mut record = IntRecord::new();
    let mut blocksize: u64 = 230;

    if args[1] == "-b" {
        blocksize = match args[2].parse() {
            Ok(num) => num,
            Err(e) => panic!("Error when parsing `-b` : {}", e.to_string()),
        };
    }

    if args[3] == "-s" {
        device =
            BlockDevice::new("tape.txt".to_string(), blocksize, true).expect("Could not create device!");
        tape = Tape::<IntRecord>::new(&mut device);

        println!("Please write single record and follow it by `return`");
        let stdin = io::stdin();
        let handle = stdin.lock();
        let lines = handle.lines();
        for line in lines {
            match record.from_string(line.expect("Could not read line")) {
                Ok(_) => record.print(),
                Err(_) => println!("Could not read the line into a record"),
            }
            tape.write_next_record(&record);
        }
    } else if args[3] == "-r" {
        device =
            BlockDevice::new("tape.txt".to_string(), blocksize, true).expect("Could not create device!");
        tape = Tape::<IntRecord>::new(&mut device);

        let num: u32 = match args[4].parse() {
            Ok(num) => num,
            Err(e) => panic!("Error when parsing `-r` : {}", e.to_string()),
        };

        for _ in 0..num {
            record
                .from_random()
                .expect("Could not generate random record");
            tape.write_next_record(&record);
        }
    } else if args[3] == "-f" {
        device = BlockDevice::new(args[4].to_string(), blocksize, false).expect("Could not open device!");
        tape = Tape::<IntRecord>::new(&mut device);
        // In order to read first buffer into memory
        tape.read_next_record();
        tape.set_head(0, 0);
    } else {
        panic!("Specify options for creation of tape!")
    }

    tape.sort();
    tape.flush();
}
