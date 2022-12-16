pub trait Bytes : PartialEq+Eq+PartialOrd+Ord{
    fn from_bytes(bytes: &[u8]) -> Self;
    fn to_bytes(&self) -> Vec<u8>;
    fn get_size() -> u64;
    fn invalid() -> Self;
}