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
    for i in vec![1,2,3,4,5] {
        record.from_bytes(vec![i;record.get_size() as usize]).unwrap();
        tape.write_next_record(&record);
    }
    tape.print();
    Ok(())
}

fn main() {
    run().expect("Error while running:");
}
