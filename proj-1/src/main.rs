pub mod record;
pub mod device;
pub mod tape;

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
    let buf = device.write(0, &vec![0xDE; 240])?;
    let mut tape = Tape::<IntRecord>::new(& mut device);
    let mut record = IntRecord::new();
    record.from_string("1 2 3 4 5 6 7 8".to_string())?;
    for i in vec![1,2,3,4,5,6] {
        tape.write_next_record(&record);
        println!("----");
        tape.print();
    }
    Ok(())
}

fn main() {
    run().expect("Error while running:");
}
