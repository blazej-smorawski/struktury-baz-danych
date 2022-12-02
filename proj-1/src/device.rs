use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

pub struct BlockDevice {
    file: File,
    pub block_size: u64,
}

impl BlockDevice {
    pub fn new(filename: String, blocksize: u64, truncate: bool) -> Result<BlockDevice, std::io::Error> {
        let file: File = OpenOptions::new().truncate(truncate).read(true).write(true).create(true).open(filename)?;
        let device = BlockDevice {
            file: file,
            block_size: blocksize,
        };
        Ok(device)
    }

    pub fn read(&mut self, buf: &mut Vec<u8>,lba: u64) -> Result<(), std::io::Error> {
        self.file.seek(SeekFrom::Start(lba * self.block_size))?;
        self.file.read_exact(buf)?;
        Ok(())
    }

    pub fn write(&mut self, lba: u64, buf: &Vec<u8>) -> Result<usize, std::io::Error> {
        if buf.len() != self.block_size as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Trying to write a buffer with a size different than devices blocksize",
            ));
        }
        self.file.seek(SeekFrom::Start(lba * self.block_size))?;
        self.file.write(buf)
    }
}
