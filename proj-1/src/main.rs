use std::{fs::File, io::{SeekFrom, Seek, Read}};

fn main(){
    let mut file = File::open("foo.txt").unwrap();
    file.seek(SeekFrom::Start(2)).unwrap();
    let aux: &mut [u8] = &mut [0; 2];
    let _buf = file.read_exact(aux);
    println!("Hello, world!");
}
