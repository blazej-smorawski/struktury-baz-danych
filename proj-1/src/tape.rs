use crate::{device::BlockDevice, record::Record};

pub struct Tape<'a, T: Record> {
    device: &'a mut BlockDevice,
    offset: u64,
    lba: u64,
    buf: Vec<u8>,
    outdated: bool,
    dirty: bool,
    record: T,
}

impl<'a, T: Record> Tape<'_, T> {
    pub fn new(device: &'a mut BlockDevice) -> Tape<T> {
        let mut tape: Tape<T> = Tape::<T> {
            device: device,
            offset: 0,
            lba: 0,
            buf: Vec::<u8>::new(),
            outdated: true,
            dirty: false,
            record: T::new(),
        };
        tape.buf.resize(tape.device.block_size as usize, 0);
        return tape;
    }

    pub fn flush(&mut self) -> Result<(), std::io::Error> {
        if self.dirty {
            self.device.write(self.lba, &self.buf)?;
        }
        Ok(())
    }

    pub fn set_head(&mut self, offset: u64, lba: u64) -> Result<(), std::io::Error> {
        if offset >= self.device.block_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Offset cannot be bigger than devices blocksize",
            ));
        }

        if lba != self.lba {
            if self.dirty {
                self.flush()?;
            }

            self.lba = lba;
            self.outdated = true;
            self.dirty = false;
        }

        self.offset = offset;
        Ok(())
    }

    fn move_head_to_next(&mut self) -> Result<(), std::io::Error> {
        let mut offset = self.offset + self.record.get_size();
        let mut lba = self.lba;
        if offset + self.record.get_size() > self.device.block_size {
            offset = 0;
            lba = self.lba + 1;
        }

        self.set_head(offset, lba)?;
        Ok(())
    }

    pub fn read_next_record(&mut self) -> Result<&T, std::io::Error> {
        if self.outdated {
            match self.device.read(&mut self.buf, self.lba) {
                Ok(_) => (),
                Err(e) => return Err(e),
            };
            self.outdated = false;
        }

        let src = self.offset as usize;
        let len = self.record.get_size() as usize;
        self.record.from_bytes(self.buf[src..src + len].to_vec())?;

        match self.move_head_to_next() {
            Ok(_) => (),
            Err(e) => return Err(e),
        };
        return Ok(&self.record);
    }

    pub fn write_next_record(&mut self, record: &T) -> () {
        self.dirty = true;
        let src = record.get_bytes();
        let off = self.offset as usize;
        let len = self.record.get_size() as usize;
        let dst = &mut self.buf[off..off + len];
        dst.copy_from_slice(&src);

        self.move_head_to_next();
    }

    pub fn print(&mut self) -> Result<(), std::io::Error> {
        let old_offset = self.offset;
        let old_lba = self.lba;
        let old_buf = self.buf.clone();
        let old_outdated = self.outdated;
        let old_dirty = self.dirty;
        let old_record = self.record.get_bytes();

        self.set_head(0, 0)?;
        loop {
            match self.read_next_record() {
                Ok(record) => record.print(),
                Err(e) => {
                    println!("{}", e);
                    break;
                }
            };
        }

        self.offset = old_offset;
        self.lba = old_lba;
        self.buf.copy_from_slice(&old_buf);
        self.outdated = old_outdated;
        self.dirty = old_dirty;
        self.record.from_bytes(old_record)?;
        Ok(())
    }
}
