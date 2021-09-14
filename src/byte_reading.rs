use std::io::{Read, Seek, SeekFrom};

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use cgmath::{Vector3, Vector4};

/// Reads a fixed size string from the reader. The size is given in bytes (i.e. not the length of the string!).
pub fn read_fixed_string<T: Read>(mut input: T, size: usize) -> Result<String> {
    let mut buf: Vec<u8> = vec![0; size];
    input.read_exact(&mut buf)?;
    let string = std::str::from_utf8(&buf)?;
    Ok(string.to_string())
}

/// The VersionHeader is actually the magic four bytes at the start of most Telltale-files.
#[derive(Debug)]
pub enum VersionHeader {
    MBIN,
    MTRE,
    MSV5,
    MSV6,
    Unknown(u32),
}
impl VersionHeader {
    pub fn parse<T: Read>(mut input: T) -> Result<Self> {
        match input.read_u32::<LittleEndian>()? {
            1296189774 => Ok(Self::MBIN),
            1297371717 => Ok(Self::MTRE),
            1297307189 => Ok(Self::MSV5),
            1297307190 => Ok(Self::MSV6),
            value => Ok(Self::Unknown(value)),
        }
    }
}

pub fn parse_vec3_f32<T: Read>(mut input: T) -> Result<Vector3<f32>> {
    let x = input.read_f32::<LittleEndian>()?;
    let y = input.read_f32::<LittleEndian>()?;
    let z = input.read_f32::<LittleEndian>()?;
    Ok(Vector3 { x, y, z })
}

pub fn parse_vec4_f32<T: Read>(mut input: T) -> Result<Vector4<f32>> {
    let x = input.read_f32::<LittleEndian>()?;
    let y = input.read_f32::<LittleEndian>()?;
    let z = input.read_f32::<LittleEndian>()?;
    // TODO does W have inverted sign (i.e. * -1)?
    let w = input.read_f32::<LittleEndian>()?;
    Ok(Vector4 { x, y, z, w })
}

#[derive(Debug, Clone)]
pub struct D3DName(String);

impl D3DName {
    pub fn parse<T: Read + Seek>(mut input: T) -> Result<Self> {
        let header_length = input.read_u32::<LittleEndian>()?;
        let mut name_length = input.read_u32::<LittleEndian>()?;
        if name_length > header_length {
            // quietly fixing offsets
            input.seek(SeekFrom::Current(-0x04))?;
            name_length = header_length;
        }
        let name = read_fixed_string(input, name_length as usize)?;
        Ok(Self(name))
    }

    pub fn to_string(self) -> String {
        self.0
    }
}
