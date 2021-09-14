use std::collections::HashMap;

use crc::{Crc, CRC_64_ECMA_182};

pub struct ChecksumMap(HashMap<u64, String>);

impl ChecksumMap {
    pub fn new() -> Self {
        let strings = include_str!("../checksum_mapping/strings.txt");

        let crc = Crc::<u64>::new(&CRC_64_ECMA_182);
        let mut map = HashMap::new();
        strings.lines().for_each(|string| {
            let mut digest = crc.digest();
            digest.update(string.as_bytes());
            let crc64 = digest.finalize();
            map.insert(crc64, string.to_string());
        });

        Self(map)
    }

    pub fn get_mapping(&self, hash: u64) -> Option<String> {
        match self.0.get(&hash) {
            Some(val) => Some(val.to_string()),
            None => None,
        }
    }
}
