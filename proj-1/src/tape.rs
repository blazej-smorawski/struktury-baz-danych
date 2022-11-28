use crate::{device::BlockDevice, record::Record};

pub struct Tape<'a, T:Record>{
    device: &'a  mut BlockDevice,
    offset: u64,
    lba: u64,
    record: T,
}

impl<'a, T:Record> Tape<'_, T>{
    pub fn new(device: &'a mut BlockDevice) -> Tape<T> {
        let tape: Tape<T> = Tape::<T> {
            device: device,
            offset: 0,
            lba:0,
            record: T::new() 
        };
        return tape;
    }

    fn move_head_to_next(& mut self) -> Result<(), std::io::Error> {
        self.offset += self.record.get_size();
        if self.offset + self.record.get_size() > self.device.block_size {
            if self.device.dirty {
                self.device.write_loaded(self.lba)?;
            }
            self.offset = 0;
            self.lba += 1;
            self.device.read(self.lba)?;
        }
        Ok(())
    }

    pub fn read_next_record(& mut self) -> Result<&T, std::io::Error> {
        self.move_head_to_next();

        self.record.from_bytes(self.device.block[(self.offset as usize) .. (self.record.get_size() as usize)].to_vec())?;

        return Ok(&self.record)
    }

    pub fn write_next_record(& mut self, record: &T) -> () {
        self.move_head_to_next();
        self.device.dirty = true;
        let src = record.get_bytes();
        let off = self.offset as usize;
        let len = self.record.get_size() as usize;
        let dst = & mut self.device.block[off .. off+len];
        dst.copy_from_slice(&src);
    
    }
}