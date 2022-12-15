use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

pub struct BlockDevice {
    file: File,
    pub block_size: u64,
    pub reads: u64,
    pub writes: u64,
}

impl BlockDevice {
    pub fn new(
        filename: String,
        blocksize: u64,
        truncate: bool,
    ) -> Result<BlockDevice, std::io::Error> {
        let file: File = OpenOptions::new()
            .truncate(truncate)
            .read(true)
            .write(true)
            .create(true)
            .open(filename)?;
        let device = BlockDevice {
            file: file,
            block_size: blocksize,
            reads: 0,
            writes: 0,
        };
        Ok(device)
    }

    pub fn read_internal(&mut self, lba: u64) -> Result<Vec<u8>, std::io::Error> {
        let mut buf = vec![0u8; self.block_size as usize];
        self.file.seek(SeekFrom::Start(lba * self.block_size))?;
        self.file.read_exact(&mut buf)?;
        Ok((buf))
    }

    pub fn read(&mut self, lba: u64) -> Result<(Vec<u8>), std::io::Error> {
        self.reads += 1;
        self.read_internal(lba)
    }

    pub fn write_internal(&mut self, lba: u64, buf: &Vec<u8>) -> Result<usize, std::io::Error> {
        if buf.len() != self.block_size as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Trying to write a buffer with a size different than devices blocksize",
            ));
        }
        self.file.seek(SeekFrom::Start(lba * self.block_size))?;
        self.file.write(buf)
    }

    pub fn write(&mut self, lba: u64, buf: &Vec<u8>) -> Result<usize, std::io::Error> {
        self.writes += 1;
        self.write_internal(lba, buf)
    }
}
