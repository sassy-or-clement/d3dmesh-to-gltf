use std::io::{Read, Seek, SeekFrom};

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};

use crate::byte_reading::{D3DName, VersionHeader};

/// The extracted data from a .d3dmesh file
pub struct D3DHeader {
    name: String,
    version: u8,
}

impl D3DHeader {
    pub fn parse<T: Read + Seek>(mut input: T) -> Result<Self> {
        let header = VersionHeader::parse(&mut input)?;
        match header {
            VersionHeader::MSV5 | VersionHeader::MSV6 => {
                let _file_size = input.read_u32::<LittleEndian>()?;
                input.seek(SeekFrom::Current(0x08))?;
                let param_count = input.read_u32::<LittleEndian>()?;
                for _ in 0..param_count {
                    input.seek(SeekFrom::Current(0x0C))?;
                }
            }
            value => return Err(anyhow!("unknown header format {:?}", value)),
        }
        let name = D3DName::parse(&mut input)?;
        let version = input.read_u8()?;
        Ok(Self {
            name: name.to_string(),
            version,
        })
    }

    /// Get a reference to the d3d header's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get a reference to the d3d header's version.
    pub fn version(&self) -> u8 {
        self.version
    }
}
