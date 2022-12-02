use std::iter;

use colored::Colorize;

use crate::{
    device::BlockDevice,
    help,
    record::{self, IntRecord, Record},
};

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

    pub fn read_next_record(&mut self) -> Option<T> {
        if self.outdated {
            match self.device.read(&mut self.buf, self.lba) {
                Ok(_) => (),
                Err(_) => return None,
            };
            self.outdated = false;
        }

        let src = self.offset as usize;
        let len = self.record.get_size() as usize;
        match self.record.from_bytes(self.buf[src..src + len].to_vec()) {
            Ok(_) => (),
            Err(_) => return None,
        }


        self.move_head_to_next();
        return Some(self.record);
    }

    pub fn write_next_record(&mut self, record: &T) {
        self.dirty = true;
        // This becomes actual version
        self.outdated = false;
        let src = record.get_bytes();
        let off = self.offset as usize;
        let len = self.record.get_size() as usize;
        let dst = &mut self.buf[off..off + len];
        dst.copy_from_slice(&src);

        self.move_head_to_next();
    }

    pub fn split(&mut self, helper: &mut Tape<T>, other_helper: &mut Tape<T>) -> u64 {
        println!(
            "{}",
            format!("-------======Split tape======-------").green()
        );

        let mut series: u64 = 0;
        let mut previous_record = None;

        while let Some(record) = self.read_next_record() {
            if Some(record) < previous_record {
                series += 1;
            }

            if series % 2 == 0 {
                helper.write_next_record(&record);
            } else {
                other_helper.write_next_record(&record);
            }

            previous_record = Some(record);
        }
        
        println!(
            "{}",
            format!("-------________Tape 1________-------").blue()
        );
        helper.print();
        println!(
            "{}",
            format!("-------________Tape 1________-------").blue()
        );
        other_helper.print();
        println!("{}", format!("Series -> {}", series).bright_blue().bold().blink());

        series
    }

    pub fn join(&mut self, helper: &mut Tape<T>, other_helper: &mut Tape<T>) -> u64 {
        println!(
            "{}",
            format!("-------======Join helper tapes======-------").green()
        );

        let mut series: u64 = 1;

        self.set_head(0, 0);
        helper.set_head(0, 0);
        other_helper.set_head(0, 0);

        let mut previous: Option<T> = None;

        let mut first = helper.read_next_record();
        let mut second = other_helper.read_next_record();

        loop {
            let heads = vec![first, second];
            let min_option = heads
                .iter()
                .flatten()
                .filter(|&x| Some(x) >= previous.as_ref())
                .min();

            let record;

            if min_option.is_none() {
                // Look for min without condition on `previous`
                let other_min_option = heads.iter().flatten().min();

                if let Some(min) = other_min_option {
                    series += 1;
                    record = min;
                } else {
                    // No records left
                    break;
                }
            } else {
                record = min_option.unwrap();
            }

            self.write_next_record(record);
            if Some(record) == first.as_ref() {
                first = helper.read_next_record();
            } else if Some(record) == second.as_ref() {
                second = other_helper.read_next_record();
            }
            previous = Some(*record);
        }

        self.print();
        series
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
            while off + len < self.device.block_size as usize {
                let slice = buf[off..off + len].to_vec();
                match rec.from_bytes(slice) {
                    Ok(_) => (),
                    Err(_) => break,
                };
                rec.print();
                off += len;
            }

            lba += 1;
        }
    }
}
