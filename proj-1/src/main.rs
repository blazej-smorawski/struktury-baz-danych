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
    let mut device = BlockDevice::new("new5.txt".to_string(), 256)?;
    let buf = device.write(0, &vec![0xDE; 256])?;
    let mut tape = Tape::<IntRecord>::new(& mut device);
    let mut record = IntRecord::new();
    record.from_string("1 2 3 4 5 6 7 8".to_string())?;
    for i in vec![1,2,3,4,5,6] {
        tape.write_next_record(&record);
    }
    println!("{:?}", buf);
    Ok(())
}

fn main() {
    run().expect("Error while running:");
}
