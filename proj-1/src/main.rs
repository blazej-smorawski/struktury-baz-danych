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

    if args[1] == "-s" {
        device =
            BlockDevice::new("tape.txt".to_string(), 240, true).expect("Could not create device!");
        tape = Tape::<IntRecord>::new(&mut device);

        println!("Please write single record and follow it by `return`");
        let stdin = io::stdin();
        let handle = stdin.lock();
        let lines = handle.lines();
        for line in lines {
            match record.from_string(line.expect("Could not read line")) {
                Ok(_) => (),
                Err(_) => println!("Could not read the line into a record"),
            }
            tape.write_next_record(&record);
        }
    } else if args[1] == "-r" {
        device =
            BlockDevice::new("tape.txt".to_string(), 240, true).expect("Could not create device!");
        tape = Tape::<IntRecord>::new(&mut device);

        let num: u32 = match args[2].parse() {
            Ok(num) => num,
            Err(e) => panic!("Error when parsing `-r` : {}", e.to_string()),
        };

        for _ in 0..num {
            record
                .from_random()
                .expect("Could not generate random record");
            tape.write_next_record(&record);
        }
    } else if args[1] == "-f" {
        device = BlockDevice::new(args[2].to_string(), 240, false).expect("Could not open device!");
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
