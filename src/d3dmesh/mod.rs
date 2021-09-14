mod header;
pub mod materials;
pub mod mesh;
pub mod polygons;
pub mod textures;

use std::io::{Read, Seek, SeekFrom};

use anyhow::{anyhow, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use cgmath::Vector3;

use crate::{
    checksum_mapping::ChecksumMap,
    d3dmesh::{
        header::D3DHeader,
        mesh::{ModelClamps, ModelOrientation, UVClamps, UV},
    },
};

use self::{mesh::Mesh, polygons::PolygonInfo};

#[derive(Debug)]
pub struct Data {
    pub materials: Vec<materials::Material>,
    pub mesh: Mesh,
    pub polygons: Vec<PolygonInfo>,
}

impl Data {
    pub fn parse<T: Read + Seek>(mut input: T, texture_mapping: &ChecksumMap) -> Result<Self> {
        let header = D3DHeader::parse(&mut input).context("could not parse D3D header")?;
        log::debug!(
            "Importing {} (Version {})...",
            header.name(),
            header.version()
        );

        if header.version() != 55 {
            return Err(anyhow!("unsupported version {}!", header.version()));
        }
        log::debug!(
            "Section 1 (Model info) start = {:#X}",
            input.stream_position()?
        );
        input.seek(SeekFrom::Current(0x14))?;

        log::debug!(
            "Section 2 (Material info) start = {:#X}",
            input.stream_position()?
        );
        let material_count = input.read_u32::<LittleEndian>()?;
        log::debug!("Material Count = {}", material_count);
        let mut materials = Vec::new();
        for id in 0..material_count {
            log::debug!("Material #{} start = {:#X}", id, input.stream_position()?);
            let material = materials::Material::parse(&mut input, id, texture_mapping)?;
            materials.push(material);
        }
        // skip unknown bytes
        input.seek(SeekFrom::Current(0x05))?;

        let face_data_start = input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;

        log::debug!(
            "Section 3 (LOD info) start = {:#X}",
            input.stream_position()?,
        );
        let mut polygons = polygons::PolygonInfo::parse(&mut input)
            .context("could not parse polygon information")?;

        {
            log::debug!("Section 4 (Empty?) start = {:#X}", input.stream_position()?);
            let section_4_end = input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;
            input.seek(SeekFrom::Start(section_4_end))?;
        }

        let material_groups = {
            log::debug!(
                "Section 5 (Material Groups) start = {:#X}",
                input.stream_position()?
            );
            let section_5_end = input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;
            let material_group_count = input.read_u32::<LittleEndian>()?;
            let mut material_groups = Vec::new();
            for _ in 0..material_group_count {
                let material_group = materials::MaterialGroup::parse(&mut input)?;
                material_groups.push(material_group);
            }
            input.seek(SeekFrom::Start(section_5_end))?;
            material_groups
        };

        {
            log::debug!("Section 6 start = {:#X}", input.stream_position()?);
            let section_6_end = input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;
            input.seek(SeekFrom::Start(section_6_end))?;
        }

        let bone_ids = {
            log::debug!(
                "Section 7 (Bone IDs) start = {:#X}",
                input.stream_position()?
            );
            let section_7_end = input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;

            let mut bone_ids = Vec::new();
            let bone_ids_count = input.read_u32::<LittleEndian>()?;
            for _ in 0..bone_ids_count {
                let bone_id = input.read_u64::<LittleEndian>()?;
                bone_ids.push(bone_id);
                // skip unknown bytes
                input.seek(SeekFrom::Current(0x30))?;
            }

            input.seek(SeekFrom::Start(section_7_end))?;
            bone_ids
        };

        {
            log::debug!("Section 8 (Empty?) start = {:#X}", input.stream_position()?);
            let section_8_end = input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;
            input.seek(SeekFrom::Start(section_8_end))?;
        }

        {
            log::debug!("Section 9 start = {:#X}", input.stream_position()?);
            let section_9_end = input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;
            input.seek(SeekFrom::Start(section_9_end))?;
        }

        // model clamps are used to define a range in which the position-values are valid
        // this is used to compress position data, because depending on the size of the clamps,
        // 16-bit values for floats are enough. These are then scaled back with those clamps.
        let model_clamps = {
            log::debug!(
                "Section 10 (Model Clamps) start = {:#X}",
                input.stream_position()?
            );
            input.seek(SeekFrom::Current(0x08))?;
            let mesh_x_min = input.read_f32::<LittleEndian>()?;
            let mesh_y_min = input.read_f32::<LittleEndian>()?;
            let mesh_z_min = input.read_f32::<LittleEndian>()?;
            let mesh_x_max = input.read_f32::<LittleEndian>()?;
            let mesh_y_max = input.read_f32::<LittleEndian>()?;
            let mesh_z_max = input.read_f32::<LittleEndian>()?;

            input.seek(SeekFrom::Current(0x24))?;
            let mesh_float_x = input.read_f32::<LittleEndian>()?;
            let mesh_float_y = input.read_f32::<LittleEndian>()?;
            let mesh_float_z = input.read_f32::<LittleEndian>()?;
            let mut orientation = ModelOrientation::Q;
            if mesh_float_x != 0.0 {
                orientation = ModelOrientation::X;
            }
            if mesh_float_y != 0.0 {
                orientation = ModelOrientation::Y;
            }
            if mesh_float_z != 0.0 {
                orientation = ModelOrientation::Z;
            }

            let clamps = ModelClamps {
                mesh_min: Vector3 {
                    x: mesh_x_min,
                    y: mesh_y_min,
                    z: mesh_z_min,
                },
                mesh_multiplier: Vector3 {
                    x: mesh_x_max - mesh_x_min,
                    y: mesh_y_max - mesh_y_min,
                    z: mesh_z_max - mesh_z_min,
                },
                orientation,
            };
            input.seek(SeekFrom::Current(0x18))?;
            //input.seek(SeekFrom::Current(0x48))?;
            clamps
        };

        let (vert_count, vert_flags) = {
            log::debug!("Section 11 start = {:#X}", input.stream_position()?);

            let vert_count = input.read_u32::<LittleEndian>()?;
            let vert_flags = input.read_u32::<LittleEndian>()?;

            let section_11a_end =
                input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;
            input.seek(SeekFrom::Start(section_11a_end))?;

            (vert_count, vert_flags)
        };

        let uv_clamps = {
            log::debug!(
                "Section 11B (UV Clamps) start = {:#X}",
                input.stream_position()?
            );
            let uv_layer_count = input.read_u32::<LittleEndian>()?;
            let mut uv_clamps = [None, None, None, None, None, None];
            for _ in 0..uv_layer_count {
                let uv_layer = input.read_u32::<LittleEndian>()?;
                let u_multiplier = input.read_f32::<LittleEndian>()?;
                let v_multiplier = input.read_f32::<LittleEndian>()?;
                let u_start = input.read_f32::<LittleEndian>()?;
                let v_start = input.read_f32::<LittleEndian>()?;
                uv_clamps[uv_layer as usize] = Some(UVClamps {
                    multiplier: UV {
                        u: u_multiplier,
                        v: v_multiplier,
                    },
                    start: UV {
                        u: u_start,
                        v: v_start,
                    },
                });
            }
            uv_clamps
        };

        let vert_start = {
            log::debug!("Section 11C start = {:#X}", input.stream_position()?);
            let mut vert_start = 0;
            if vert_flags == 0x31 {
                // skip unknowns
                input.seek(SeekFrom::Current(0x24))?;
                let vertex_parameter_start =
                    input.stream_position()? + input.read_u32::<LittleEndian>()? as u64;
                let _vertex_buffer_size = input.read_u32::<LittleEndian>()?;
                vert_start = input.stream_position()?;
                input.seek(SeekFrom::Start(vertex_parameter_start))?;
            }
            vert_start
        };

        log::debug!(
            "Section 12 (Vertex/Face Buffer Info) start = {:#X} vert_count = {}",
            input.stream_position()?,
            vert_count,
        );
        let mesh = mesh::Mesh::parse(
            &mut input,
            face_data_start,
            vert_start,
            vert_flags,
            vert_count,
            &model_clamps,
            &uv_clamps,
            &bone_ids,
        )?;

        fix_material_index(&mut polygons, &material_groups, &materials)?;

        Ok(Self {
            materials,
            mesh,
            polygons,
        })
    }
}

/// Changes the material number according to the reference in the material groups and materials section.
/// I.e. PolygonInfo references an index of the MaterialGroup which in turn references the ID of the material.
/// The index in the PolygonInfo is then changed to the index of the Material instead of the material group.
fn fix_material_index(
    polygons: &mut [PolygonInfo],
    material_groups: &[materials::MaterialGroup],
    materials: &[materials::Material],
) -> Result<()> {
    let get_material_index_for_material_group_index = |material_group_index: u32| {
        let material_id = material_groups[material_group_index as usize].material_id;
        let mut material_index = None;
        for (index, material) in materials.iter().enumerate() {
            if material.material_id == material_id {
                material_index = Some(index as u32);
            }
        }
        material_index
    };

    for polygon in polygons {
        let old_index = polygon.mat_num;
        let new_index = get_material_index_for_material_group_index(old_index).ok_or(anyhow!(
            "could not determine the material index due to missing material group reference"
        ))?;
        polygon.mat_num = new_index;
    }
    Ok(())
}
