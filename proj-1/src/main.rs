pub mod record;
pub mod device;
pub mod tape;

use std::fmt::Display;

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

fn main() {
    run().expect("Error while running:");
}
