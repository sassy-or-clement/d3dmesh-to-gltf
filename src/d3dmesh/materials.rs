use std::io::{Read, Seek, SeekFrom};

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};

use crate::checksum_mapping::ChecksumMap;

use super::textures::Texture;

#[derive(Debug)]
pub struct Material {
    pub textures: Vec<Texture>,
    pub material_id: u64,
}

impl Material {
    pub fn parse<T: Read + Seek>(
        mut input: T,
        index: u32,
        texture_mapping: &ChecksumMap,
    ) -> Result<Self> {
        let mut textures = Vec::new();

        let material_id = input.read_u64::<LittleEndian>()?;
        let _unk_hash_2 = input.read_u32::<LittleEndian>()?;
        let _unk_hash_1 = input.read_u32::<LittleEndian>()?;
        let material_header_size = input.read_u32::<LittleEndian>()?;
        // the end of the material section can be used to seek to the end of the section
        let material_section_end = input.stream_position()? as u32 + material_header_size - 4;
        log::debug!("should end = {:#X}", material_section_end);

        let _mat_unk_1 = input.read_u32::<LittleEndian>()?;
        let _mat_unk_2 = input.read_u32::<LittleEndian>()?;
        let _mat_header_size_b = input.read_u32::<LittleEndian>()?;

        let mat_unk_3_count = input.read_u32::<LittleEndian>()?;
        for _ in 0..mat_unk_3_count {
            let _mat_unk_3_hash_2 = input.read_u32::<LittleEndian>()?;
            let _mat_unk_3_hash_1 = input.read_u32::<LittleEndian>()?;
        }

        let mat_param_count = input.read_u32::<LittleEndian>()?;
        log::debug!("Material parameter count = {}", mat_param_count);
        for _ in 0..mat_param_count {
            let mat_section_hash = input.read_u64::<LittleEndian>()?;
            let mat_section_count = input.read_u32::<LittleEndian>()?;
            log::debug!(
                "Material hash: {:016x}, Count = {}, Offset = {:#X}",
                mat_section_hash,
                mat_section_count,
                input.stream_position()?
            );

            match mat_section_hash {
                0x0000000000000000 => {} // ...Nothing?
                0xa98f0652295de685 => {} // ...Nothing?
                0xfa21e4c88ae64d31 => {} // ...Nothing?
                0x254edc517b59bb47 => {} // ...Nothing?
                0x7caceebcd26d075c => {} // ...Nothing?
                0xded5e1937b1689ef => {} // ...Nothing?
                0x264ac2f2544e517c => {
                    // Hacky fix for "adv_boardingSchoolExterior_meshesABuilding" to prevent erroring.
                    input.seek(SeekFrom::Current(-0x04))?;
                }
                0x873c2f1835428297 => {
                    // Hacky fix for "obj_vehicleTruckForestShack" to prevent erroring.
                    input.seek(SeekFrom::Current(0x08))?;
                }
                0x4e7d91f16f97a3c2 => {
                    // Hacky fix for "ui_icon" to prevent erroring.
                    input.seek(SeekFrom::Current(-0x04))?;
                }
                0x181afb3ebb8f90ae => {
                    // Hacky fix for "ui_icon" to prevent erroring.
                }
                0xfec9ffdf25b43917 => {
                    // Hacky fix for "ui_mask" to prevent erroring.
                    input.seek(SeekFrom::Current(-0x04))?;
                }
                0x8c44858f42cd32d5 => {
                    // Hacky fix for "ui_mask" to prevent erroring.
                }
                0xb76e07d6bb899bfe => {
                    for _ in 0..mat_section_count {
                        // Four floats (alternate?)
                        let _unknown_hash_2 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_1 = input.read_u32::<LittleEndian>()?;
                        let _unknown_float_1 = input.read_f32::<LittleEndian>()?;
                        let _unknown_float_2 = input.read_f32::<LittleEndian>()?;
                        let _unknown_float_3 = input.read_f32::<LittleEndian>()?;
                        let _unknown_float_4 = input.read_f32::<LittleEndian>()?;
                    }
                }
                0x004f023463d89fb0 => {
                    for _ in 0..mat_section_count {
                        // One hash set
                        let _unknown_hash_2 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_1 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_4 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_3 = input.read_u32::<LittleEndian>()?;
                    }
                }
                0xbae4cbd77f139a91 => {
                    for _ in 0..mat_section_count {
                        // One float
                        let _unknown_hash_2 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_1 = input.read_u32::<LittleEndian>()?;
                        let _unknown_float_1 = input.read_f32::<LittleEndian>()?;
                    }
                }
                0x9004c5587575d6c0 => {
                    for _ in 0..mat_section_count {
                        // One byte, boolean?
                        let _unknown_hash_2 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_1 = input.read_u32::<LittleEndian>()?;
                        let _unknown_byte_1 = input.read_u8()?;
                    }
                }
                0x394c43af4ff52c94 => {
                    for _ in 0..mat_section_count {
                        // Three floats
                        let _unknown_hash_2 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_1 = input.read_u32::<LittleEndian>()?;
                        let _unknown_float_1 = input.read_f32::<LittleEndian>()?;
                        let _unknown_float_2 = input.read_f32::<LittleEndian>()?;
                        let _unknown_float_3 = input.read_f32::<LittleEndian>()?;
                    }
                }
                0x7bbca244e61f1a07 => {
                    for _ in 0..mat_section_count {
                        // Two floats
                        let _unknown_hash_2 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_1 = input.read_u32::<LittleEndian>()?;
                        let _unknown_float_1 = input.read_f32::<LittleEndian>()?;
                        let _unknown_float_2 = input.read_f32::<LittleEndian>()?;
                    }
                }
                0xc16762f7763d62ab => {
                    for _ in 0..mat_section_count {
                        // Four floats
                        let _unknown_hash_2 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_1 = input.read_u32::<LittleEndian>()?;
                        let _unknown_float_1 = input.read_f32::<LittleEndian>()?;
                        let _unknown_float_2 = input.read_f32::<LittleEndian>()?;
                        let _unknown_float_3 = input.read_f32::<LittleEndian>()?;
                        let _unknown_float_4 = input.read_f32::<LittleEndian>()?;
                    }
                }
                0x52a09151f1c3f2c7 => {
                    log::debug!("Material #{}, uses the following textures:", index);
                    for _ in 0..mat_section_count {
                        let texture = Texture::parse(&mut input, texture_mapping)?;
                        textures.push(texture);
                    }
                }
                0xe2ba743e952f9338 => {
                    for _ in 0..mat_section_count {
                        // Two hash sets
                        let _unknown_hash_2 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_1 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_4 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_3 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_6 = input.read_u32::<LittleEndian>()?;
                        let _unknown_hash_5 = input.read_u32::<LittleEndian>()?;
                    }
                }
                _ => {
                    log::warn!("Warning: unknown material hash {:016x}", mat_section_hash);
                    break;
                    //return Err(anyhow!("unknown material hash {:016x}", mat_section_hash))
                }
            };
        }

        input.seek(SeekFrom::Start(material_section_end as u64))?;

        Ok(Self {
            textures,
            material_id,
        })
    }
}

/// A material groups holds a reference to a specific material.
#[derive(Debug)]
pub struct MaterialGroup {
    pub material_id: u64,
}

impl MaterialGroup {
    pub fn parse<T: Read + Seek>(mut input: T) -> Result<Self> {
        let _unknown = input.read_u32::<LittleEndian>()?;
        let material_id = input.read_u64::<LittleEndian>()?;
        // skip unknowns
        input.seek(SeekFrom::Current(0x40))?;
        Ok(Self { material_id })
    }
}
