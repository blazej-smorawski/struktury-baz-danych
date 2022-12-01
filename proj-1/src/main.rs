pub mod record;
pub mod device;
pub mod tape;
use std::io::BufRead;
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
        let mut handle = stdin.lock();
        let lines = handle.lines();
        for line in lines {
            match record.from_string(line.expect("Could not read line")) {
                Ok(_) => (),
                Err(_) => println!("Could not read the line into a record")
            }
            tape.write_next_record(&record);
        }

        tape.print();
    }

}
