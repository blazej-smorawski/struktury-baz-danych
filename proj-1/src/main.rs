use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

struct BlockDevice {
    file: File,
    block_size: usize,
    block: Vec<u8>,
}

impl BlockDevice {
    pub fn new(filename: String, blocksize: usize) -> Result<BlockDevice, std::io::Error> {
        let file: File = OpenOptions::new()
            .write(true)
            .create(true)
            .open(filename)?;
        let mut device = BlockDevice {
            file: file,
            block_size: blocksize,
            block: Vec::<u8>::new(),
        };
        device.block.resize(device.block_size, 0u8);
        Ok(device)
    }

    pub fn read(&mut self, offset: u64) -> Result<&Vec<u8>, std::io::Error> {
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut self.block)?;
        Ok(&self.block)
    }

    pub fn write(&mut self, offset: u64, buf: &Vec<u8>) -> Result<usize, std::io::Error> {
        if buf.len() != self.block_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Trying to write a buffer with a size different than devices blocksize",
            ));
        }
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write(buf)
    }
}

fn run() -> Result<(), std::io::Error> {
    let mut device = BlockDevice::new("new5.txt".to_string(), 2)?;
    let buf = device.write(20, &vec![0xDE, 0xDE, 0xAB])?;
    println!("{:?}", buf);
    Ok(())
}

fn main() {
    run().expect("Error while running:");
}
