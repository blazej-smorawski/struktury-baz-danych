use crate::{device::BlockDevice, record::{Record, IntRecord}};

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

    pub fn flush(&mut self) {
        if self.dirty {
            self.device
                .write(self.lba, &self.buf)
                .expect("Could not write block");
            self.dirty = false;
        }
    }

    pub fn set_head(&mut self, offset: u64, lba: u64) {
        if offset >= self.device.block_size {
            panic!("Wrong offset");
        }

        if lba != self.lba {
            if self.dirty {
                self.flush();
            }
            self.buf.fill(0);
            self.lba = lba;
            self.outdated = true;
            self.dirty = false;
        }

        self.offset = offset;
    }

    fn move_head_to_next(&mut self) {
        let mut offset = self.offset + self.record.get_size();
        let mut lba = self.lba;
        if offset + self.record.get_size() > self.device.block_size {
            offset = 0;
            lba = self.lba + 1;
        }

        self.set_head(offset, lba);
    }

    pub fn read_next_record(&mut self) -> Result<T, std::io::Error> {
        if self.outdated {
            match self.device.read(&mut self.buf, self.lba) {
                Ok(_) => (),
                Err(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Block not found",
                    ))
                }
            };
            self.outdated = false;
        }

        let src = self.offset as usize;
        let len = self.record.get_size() as usize;
        self.record
            .from_bytes(self.buf[src..src + len].to_vec())
            .expect("Could not create record from the bytes");

        self.move_head_to_next();
        return Ok(self.record);
    }

    pub fn write_next_record(&mut self, record: &T) {
        self.dirty = true;
        let src = record.get_bytes();
        let off = self.offset as usize;
        let len = self.record.get_size() as usize;
        let dst = &mut self.buf[off..off + len];
        dst.copy_from_slice(&src);

        self.move_head_to_next();
    }

    pub fn print(&mut self) {
        let mut buf = vec![0; self.device.block_size as usize];
        let mut lba: u64 = 0;

        loop {
            println!("BLOCK #{}", lba);
            if self.lba == lba {
                buf.copy_from_slice(&self.buf);
            } else {
                match self.device.read(&mut buf, lba) {
                    Ok(_) => (),
                    Err(_) => break,
                };
            }

            let mut off: usize = 0;
            let len: usize = self.record.get_size() as usize;
            let mut rec: T = T::new();
            while off + len < self.device.block_size as usize{
                let slice = buf[off..off + len].to_vec();
                rec.from_bytes(slice).unwrap();
                rec.print();
                off += len;
            }

            lba += 1;
        }
    }
}
