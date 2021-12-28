use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use bincode;
use bincode::Options;

pub fn serialize_bytes<T: Serialize>(data: &T) -> Vec<u8> {
    bincode::serialize(data).unwrap()
}
pub fn deserialize_bytes<T: DeserializeOwned>(bytes: &[u8]) -> T {
    bincode::deserialize(bytes).unwrap()
}

pub fn save_to_disk<T: Serialize, TPath: AsRef<Path>>(data: &T, path: TPath) {
    let options = bincode::DefaultOptions::new();
    let options = options.with_no_limit();
    // Write all bytes to the target file
    let file = File::create(path).unwrap();
    let writer = BufWriter::new(file);
    options.serialize_into(writer, data).unwrap();
}

pub fn load_from_disk<T: DeserializeOwned, TPath: AsRef<Path>>(path: TPath) -> T {
    // Open the file and read all bytes
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let options = bincode::DefaultOptions::new();
    let options = options.with_no_limit();
    options.deserialize_from(reader).unwrap()
}
