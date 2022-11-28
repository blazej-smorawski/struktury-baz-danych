use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

pub struct BlockDevice {
    file: File,
    pub block_size: u64,
    pub lba: u64,
    pub dirty: bool,
    pub block: Vec<u8>,
}

impl BlockDevice {
    pub fn new(filename: String, blocksize: u64) -> Result<BlockDevice, std::io::Error> {
        let file: File = OpenOptions::new()
            .write(true)
            .create(true)
            .open(filename)?;
        let mut device = BlockDevice {
            file: file,
            block_size: blocksize,
            lba: 0,
            dirty: false,
            block: Vec::<u8>::new(),
        };
        device.block.resize(device.block_size as usize, 0u8);
        Ok(device)
    }

    pub fn read(&mut self, lba: u64) -> Result<&Vec<u8>, std::io::Error> {
        if self.lba == lba {
            return Ok(&self.block)
        }

        self.lba = lba;
        self.file.seek(SeekFrom::Start(lba*self.block_size))?;
        self.file.read_exact(&mut self.block)?;
        Ok(&self.block)
    }

    pub fn write(&mut self, lba: u64, buf: &Vec<u8>) -> Result<usize, std::io::Error> {
        if buf.len() != self.block_size as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Trying to write a buffer with a size different than devices blocksize",
            ));
        }
        self.lba = lba;
        self.file.seek(SeekFrom::Start(lba*self.block_size))?;
        self.file.write(buf)
    }

    pub fn write_loaded(&mut self, lba: u64) -> Result<usize, std::io::Error> {
        self.lba = lba;
        self.file.seek(SeekFrom::Start(lba*self.block_size))?;
        self.file.write(&self.block)
    }
}