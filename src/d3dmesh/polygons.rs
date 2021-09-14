use std::io::{Read, Seek, SeekFrom};

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};

use crate::byte_reading::parse_vec3_f32;

#[derive(Debug)]
pub struct PolygonInfo {
    pub vertex_start: u32,
    pub vertex_min: u32,
    pub vertex_max: u32,
    pub polygon_start: u32,
    pub polygon_count: u32,
    pub face_point_count: u32,
    pub mat_num: u32,
    pub lod_level: u32,
}

impl PolygonInfo {
    pub fn parse<T: Read + Seek>(mut input: T) -> Result<Vec<Self>> {
        let mut infos = Vec::new();

        let section_3_end = input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;
        let section_3_count = input.read_u32::<LittleEndian>()?;
        for lod_level in 0..section_3_count {
            log::debug!(
                "LOD {}/{} information start = {:#X}",
                lod_level + 1,
                section_3_count,
                input.stream_position()?
            );
            let section_end = input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;
            let polygon_total = input.read_u32::<LittleEndian>()?;

            for polygon_index in 0..polygon_total {
                log::debug!(
                    "polygon information {}/{}, start = {:#X}",
                    polygon_index + 1,
                    polygon_total,
                    input.stream_position()?
                );
                let _bounding_box_min = parse_vec3_f32(&mut input)?;
                let _bounding_box_max = parse_vec3_f32(&mut input)?;
                let _header_length = input.read_u32::<LittleEndian>()?;
                // skip unknowns
                input.seek(SeekFrom::Current(20))?;
                let vertex_min = input.read_u32::<LittleEndian>()?;
                let vertex_max = input.read_u32::<LittleEndian>()?;
                let vertex_start = input.read_u32::<LittleEndian>()?;
                let face_point_start = input.read_u32::<LittleEndian>()?;
                let polygon_start = face_point_start / 3;
                let polygon_count = input.read_u32::<LittleEndian>()?;
                let face_point_count = input.read_u32::<LittleEndian>()?;
                let header_length_2 = input.read_u32::<LittleEndian>()?;
                if header_length_2 == 0x10 {
                    input.seek(SeekFrom::Current(0x08))?;
                }
                let _unknown = input.read_u32::<LittleEndian>()?;
                let mat_num = input.read_u32::<LittleEndian>()?;
                let _unknown = input.read_u32::<LittleEndian>()?;

                if lod_level == 0 {
                    infos.push(PolygonInfo {
                        vertex_start,
                        vertex_min,
                        vertex_max,
                        polygon_start,
                        polygon_count,
                        face_point_count,
                        mat_num,
                        lod_level,
                    })
                }
            }
            input.seek(SeekFrom::Start(section_end))?;

            log::debug!("Section 3B start = {:#X}", input.stream_position()?);
            let section_3b_end =
                input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;
            let polygon_2_count = input.read_u32::<LittleEndian>()?;
            for _ in 0..polygon_2_count {
                // just skip everything, similar to the polygon loop above
                input.seek(SeekFrom::Current(0x48))?;
                let header_length_2 = input.read_u32::<LittleEndian>()?;
                if header_length_2 == 0x10 {
                    input.seek(SeekFrom::Current(0x08))?;
                }
                input.seek(SeekFrom::Current(0x0C))?;
            }
            input.seek(SeekFrom::Start(section_3b_end))?;

            log::debug!("Section 3C start = {:#X}", input.stream_position()?);
            input.seek(SeekFrom::Current(0x5C))?;

            log::debug!(
                "Section 3D (Bone IDs) start = {:#X}",
                input.stream_position()?
            );
            let _id_header_length = input.read_u32::<LittleEndian>()?;
            let bone_id_total = input.read_u32::<LittleEndian>()?;
            for _ in 0..bone_id_total {
                // skip bone checksum
                input.seek(SeekFrom::Current(0x08))?;
            }
        }
        input.seek(SeekFrom::Start(section_3_end))?;

        Ok(infos)
    }
}
