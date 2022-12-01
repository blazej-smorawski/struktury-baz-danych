pub mod record;
pub mod device;
pub mod tape;
use std::io::BufRead;
use std::ptr::null;
use std::{env, io};

use crate::record::{
    Record,
    IntRecord,
};

use crate::device::{
    BlockDevice,
};

use crate::tape::Tape;

fn run() -> Result<(), std::io::Error> {
    let mut device = BlockDevice::new("tape.txt".to_string(), 240)?;
    let mut tape = Tape::<IntRecord>::new(& mut device);
    let mut record = IntRecord::new();
    
    for i in vec![1,2,3,4,5,6,7,8,9,10] {
        let mut record_string = String::new();
        for num in 1 .. i+1 {
            record_string.push_str(&(num.to_string()+" "));
        }
        record.from_string(record_string).unwrap();
        tape.write_next_record(&record);
    }
    tape.print();
    Ok(())
}

fn help() {
    println!("usage:
    -s
    -f <path>
    -r <count>");
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
        device = BlockDevice::new("tape.txt".to_string(), 240).expect("Could not create device!");
        tape = Tape::<IntRecord>::new(& mut device);

        println!("Please write single record and follow it by `return`");
        let stdin = io::stdin();
        let handle = stdin.lock();
        let lines = handle.lines();
        for line in lines {
            match record.from_string(line.expect("Could not read line")) {
                Ok(_) => (),
                Err(_) => println!("Could not read the line into a record")
            }
            tape.write_next_record(&record);
        }
        tape.set_head(0, 0);
        tape.print();
    } else if args[1] == "-r" {
        device = BlockDevice::new("tape.txt".to_string(), 240).expect("Could not create device!");
        tape = Tape::<IntRecord>::new(& mut device);

        let num: u32 = match args[2].parse() {
            Ok(num) => num,
            Err(e) => panic!("Error when parsing `-r` : {}", e.to_string()),
        };

        for _ in 0..num {
            record.from_random();
            tape.write_next_record(&record);
        }
        tape.set_head(0, 0);
        tape.print();
    } else {
        panic!("Specify options for creation of device!")
    }

    let mut helper_device1 = BlockDevice::new("helper1.txt".to_string(), 240).expect("Could not create device!");
    let mut helper_tape1 = Tape::<IntRecord>::new(& mut helper_device1);
    let mut helper_device2 = BlockDevice::new("helper2.txt".to_string(), 240).expect("Could not create device!");
    let mut helper_tape2 = Tape::<IntRecord>::new(& mut helper_device2);

    let mut helpers = vec![helper_tape1, helper_tape2];
    let mut helper_index: usize = 0;
    
    
    let record = match tape.read_next_record() {
        Ok(record) => record,
        Err(_) => panic!("No records to sort!"),
    };
    helpers[helper_index].write_next_record(&record);
    
    let mut previous_record = record;

    loop {
        let record = match tape.read_next_record() {
            Ok(record) => record,
            Err(_) => break,
        };

        if record < previous_record {
            helper_index = (helper_index + 1) % 2;

        }
        helpers[helper_index].write_next_record(&record);
        previous_record = record;
    }

    for tape in &mut helpers{
        tape.print();
    }

}
