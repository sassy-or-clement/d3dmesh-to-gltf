use std::io::{Read, Seek, SeekFrom};

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use cgmath::{InnerSpace, Vector3, Vector4};

use crate::byte_reading::parse_vec3_f32;

#[derive(Debug)]
pub struct Mesh {
    pub positions: Vec<Vector3<f32>>,
    pub normals: Vec<Vector3<f32>>,
    pub faces: Vec<Face>,
    pub uv: Vec<Vec<UV>>,
    pub bones: Vec<BoneReference>,
    pub weights: Vec<Vector4<f32>>,
}

impl Mesh {
    pub fn parse<T: Read + Seek>(
        mut input: T,
        face_data_start: u64,
        vert_start: u64,
        vert_flags: u32,
        vert_count: u32,
        model_clamps: &ModelClamps,
        uv_clamps: &UVLayerClamps,
        bone_ids: &[u64],
    ) -> Result<Self> {
        // information
        let mut has_vertex = 0;
        let mut vertex_format = 0;
        let mut has_weights = 0;
        let mut weights_format = 0;
        let mut has_bones = 0;
        let mut bones_format = 0;
        let mut has_normals = 0;
        let mut normals_format = 0;
        let mut has_tangents = 0;
        let mut tangents_format = 0;
        let mut has_binormals = 0;
        let mut binormals_format = 0;
        let mut has_uv5 = 0;
        let mut uv5_format = 0;
        let mut has_uv6 = 0;
        let mut uv6_format = 0;
        let mut has_colors = 0;
        let mut colors_format = 0;
        let mut has_colors2 = 0;
        let mut colors2_format = 0;
        let mut has_uv1 = 0;
        let mut uv1_format = 0;
        let mut has_uv2 = 0;
        let mut uv2_format = 0;
        let mut has_uv3 = 0;
        let mut uv3_format = 0;
        let mut has_uv4 = 0;
        let mut uv4_format = 0;
        let mut face_point_count = 0;
        let mut face_point_count_b = 0;
        let mut _face_length = 0;
        let mut _face_length_b = 0;
        // ------------

        let mut positions = Vec::new();
        let mut bones_infos = Vec::new();
        let mut bones = Vec::new();
        let mut faces = Vec::new();

        input.seek(SeekFrom::Current(0x08))?;
        let face_buffer_count = input.read_u32::<LittleEndian>()?;
        let buffer_count_1 = input.read_u32::<LittleEndian>()?;
        let buffer_count_2 = input.read_u32::<LittleEndian>()?;
        for _ in 0..buffer_count_1 {
            let vert_type = input.read_u32::<LittleEndian>()? + 1;
            let vert_format = input.read_u32::<LittleEndian>()? + 1;
            let vert_layer = input.read_u32::<LittleEndian>()? + 1;
            let vert_buff_num = input.read_u32::<LittleEndian>()? + 1;
            let _vert_offset = input.read_u32::<LittleEndian>()? + 1;

            match (vert_type, vert_layer) {
                (1, 1) => {
                    has_vertex = vert_buff_num;
                    vertex_format = vert_format;
                }
                (4, 1) => {
                    has_weights = vert_buff_num;
                    weights_format = vert_format;
                }
                (5, 1) => {
                    has_bones = vert_buff_num;
                    bones_format = vert_format;
                }
                (2, 1) => {
                    has_normals = vert_buff_num;
                    normals_format = vert_format;
                }
                (3, 1) => {
                    has_tangents = vert_buff_num;
                    tangents_format = vert_format;
                }
                (2, 2) => {
                    has_binormals = vert_buff_num;
                    binormals_format = vert_format;
                }
                (7, 5) => {
                    has_uv5 = vert_buff_num;
                    uv5_format = vert_format;
                }
                (7, 6) => {
                    has_uv6 = vert_buff_num;
                    uv6_format = vert_format;
                }
                (6, 1) => {
                    has_colors = vert_buff_num;
                    colors_format = vert_format;
                }
                (6, 2) => {
                    has_colors2 = vert_buff_num;
                    colors2_format = vert_format;
                }
                (7, 1) => {
                    has_uv1 = vert_buff_num;
                    uv1_format = vert_format;
                }
                (7, 2) => {
                    has_uv2 = vert_buff_num;
                    uv2_format = vert_format;
                }
                (7, 3) => {
                    has_uv3 = vert_buff_num;
                    uv3_format = vert_format;
                }
                (7, 4) => {
                    has_uv4 = vert_buff_num;
                    uv4_format = vert_format;
                }
                (type_, layer) => {
                    return Err(anyhow!(
                        "unknown vertex buffer combination type={} layer={}",
                        type_,
                        layer
                    ))
                }
            }
        }

        for i in 0..face_buffer_count {
            input.seek(SeekFrom::Current(12))?;
            let face_buff_count = input.read_u32::<LittleEndian>()?;
            let face_buff_length = input.read_u32::<LittleEndian>()?;
            match i {
                0 => {
                    face_point_count = face_buff_count;
                    _face_length = face_buff_length;
                }
                1 => {
                    face_point_count_b = face_buff_count;
                    _face_length_b = face_buff_length;
                }
                _ => unreachable!(),
            }
        }

        for _ in 0..buffer_count_2 {
            // skip
            input.seek(SeekFrom::Current(0x14))?;
        }

        input.seek(SeekFrom::Start(face_data_start))?;
        log::debug!("Facepoint Buffer A start = {:#X}", input.stream_position()?);

        let mut face_array_a = Vec::new();
        for _ in 0..(face_point_count / 3) {
            let face = Face::parse(&mut input)?;
            face_array_a.push(face);
        }
        faces.extend(face_array_a.into_iter());

        let mut face_array_b = Vec::new();
        if face_buffer_count == 2 {
            log::debug!("Facepoint Buffer B start = {:#X}", input.stream_position()?);
            for _ in 0..(face_point_count_b / 3) {
                let face = Face::parse(&mut input)?;
                face_array_b.push(face);
            }
            // ignore faces of buffer B for now...
            //faces.push(face_array_b);
        }

        match vert_flags {
            0x00 | 0x01 | 0x03 | 0x05 | 0x09 | 0x21 => (),
            0x31 => {
                let vert_start_b = input.stream_position()?;
                input.seek(SeekFrom::Start(vert_start))?;
                log::debug!("Vertex buffer A start = {:#X}", input.stream_position()?);

                for _ in 0..vert_count {
                    let (position, bone_info) = parse_position_with_bones(&mut input)?;
                    positions.push(position);
                    bones_infos.push(bone_info);
                }

                input.seek(SeekFrom::Start(vert_start_b))?;
            }
            _ => return Err(anyhow!("unknown MeshFlags combination: {}", vert_flags)),
        }

        if has_vertex > 0 {
            log::debug!(
                "Positions start = {:#X}, format = {}",
                input.stream_position()?,
                vertex_format
            );
            match vertex_format {
                4 => {
                    for _ in 0..vert_count {
                        let vector = parse_vec3_f32(&mut input)?;
                        positions.push(vector);
                    }
                }
                27 => {
                    for _ in 0..vert_count {
                        let x_u16 = input.read_u16::<LittleEndian>()?;
                        let x = ((x_u16 as f32 / 65535.0) * model_clamps.mesh_multiplier.x)
                            + model_clamps.mesh_min.x;
                        let y_u16 = input.read_u16::<LittleEndian>()?;
                        let y = ((y_u16 as f32 / 65535.0) * model_clamps.mesh_multiplier.y)
                            + model_clamps.mesh_min.y;
                        let z_u16 = input.read_u16::<LittleEndian>()?;
                        let z = ((z_u16 as f32 / 65535.0) * model_clamps.mesh_multiplier.z)
                            + model_clamps.mesh_min.z;
                        let _vq_u16 = input.read_u16::<LittleEndian>()?;
                        positions.push(Vector3 { x, y, z });
                    }
                }
                42 => {
                    // Just... why?
                    // Model has awkward vertex setup, may be incorrect?
                    // seems to be good after looking at a couple of models

                    for _ in 0..vert_count {
                        let pos_vars = input.read_u32::<LittleEndian>()?;
                        let mut x = (pos_vars & 0x3FF) as f32 / 1023.0;
                        let mut y = ((pos_vars >> 10) & 0x3FF) as f32 / 1023.0;
                        let mut z = ((pos_vars >> 20) & 0x3FF) as f32 / 1023.0;
                        match model_clamps.orientation {
                            ModelOrientation::X => x = x / 4.0 + ((pos_vars >> 30) as f32 / 4.0),
                            ModelOrientation::Y => y = y / 4.0 + ((pos_vars >> 30) as f32 / 4.0),
                            ModelOrientation::Z => z = z / 4.0 + ((pos_vars >> 30) as f32 / 4.0),
                            ModelOrientation::Q => {}
                        }
                        x = (x * model_clamps.mesh_multiplier.x) + model_clamps.mesh_min.x;
                        y = (y * model_clamps.mesh_multiplier.y) + model_clamps.mesh_min.y;
                        z = (z * model_clamps.mesh_multiplier.z) + model_clamps.mesh_min.z;
                        positions.push(Vector3 { x, y, z });
                    }

                    /*
                    for x = 1 to VertCount do (
                    PosVars = readlong f
                    vx = ((bit.and (PosVars) 0x3FF) as float / 1023)
                    vy = ((bit.and (bit.shift PosVars -10) 0x3FF) as float / 1023)
                    vz = ((bit.and (bit.shift PosVars -20) 0x3FF) as float / 1023)
                    case MeshOrient of (
                        "Q":()
                        "X":(vx = vx / 4 + ((bit.shift PosVars -30) as float / 4))
                        "Y":(vy = vy / 4 + ((bit.shift PosVars -30) as float / 4))
                        "Z":(vz = vz / 4 + ((bit.shift PosVars -30) as float / 4))
                    )
                    vx = ((vx * MeshXMult) + MeshXMin) * ModelScale
                    vy = ((vy * MeshYMult) + MeshYMin) * ModelScale
                    vz = ((vz * MeshZMult) + MeshZMin) * ModelScale
                    append AllVert_array [vx,vy,vz]
                    */
                }
                val => return Err(anyhow!("unknown position format: {}", val)),
            }
        }

        let mut weights = Vec::new();
        if has_weights > 0 {
            log::debug!(
                "Weights start = {:#X}, weights_format = {}",
                input.stream_position()?,
                weights_format
            );
            match weights_format {
                27 => {
                    for _ in 0..vert_count {
                        let weight_1_u16 = input.read_u16::<LittleEndian>()?;
                        let weight_1 = (weight_1_u16 as f32) / 65535.0;
                        let weight_2_u16 = input.read_u16::<LittleEndian>()?;
                        let weight_2 = (weight_2_u16 as f32) / 65535.0;
                        let weight_3_u16 = input.read_u16::<LittleEndian>()?;
                        let weight_3 = (weight_3_u16 as f32) / 65535.0;
                        let weight_4_u16 = input.read_u16::<LittleEndian>()?;
                        let weight_4 = (weight_4_u16 as f32) / 65535.0;
                        let vector = Vector4 {
                            x: weight_1,
                            y: weight_2,
                            z: weight_3,
                            w: weight_4,
                        };
                        weights.push(vector);
                    }
                }
                42 => {
                    // From Random T Bush:
                    // "Why fix what isn't broken?" didn't apply to Telltale, it seems.
                    // This was way too frustrating to figure out, so I'll grumble here to explain how this crap works.
                    // First, you have to read all four weight bytes as a "long" value, and then break that apart into 2/10/10/10-bit binary segments.
                    // Those are used for weights 2, 4, 3 and 2 respectively. Why is 2 listed twice? The upper 2 bits add an extra 0.125 each to the second weight's value (0.375 max).
                    // And then the three sets of 10 bits each are the weights in descending order (#4 -> #3 -> #2), and need to be divided by 1023 (0x3FF) and then again for the following:
                    // 2nd = divide by 8 (0.125 max) + 0.125/0.25/0.375 from the upper bits, 3rd = divide by 3 (0.333 max), 4th = divide by 4 (0.25 max).
                    // And finally weight #1 is the remainder, 1.0 minus #2, #3 and #4 combined.
                    // In retrospect, I can see how this works... but what, exactly, was the problem with using float values for this sorta thing again???
                    // Either way, thanks to that recycled hare model for being there, making me not want to rip out my hair.
                    /*
                    for x = 1 to VertCount do (
                        WeightVars = readlong f
                        Weight2 = (((bit.and (WeightVars) 0x3FF) as float / 1023) / 8) + ((bit.shift WeightVars -30) as float / 8)
                        Weight3 = ((bit.and (bit.shift WeightVars -10) 0x3FF) as float / 1023) / 3
                        Weight4 = ((bit.and (bit.shift WeightVars -20) 0x3FF) as float / 1023) / 4
                        Weight1 = (1 as float - Weight2 - Weight3 - Weight4)
                        append W1_array (Weight_Info_Struct Weight1:Weight1 Weight2:Weight2 Weight3:Weight3 Weight4:Weight4)
                    )
                    */
                    for _ in 0..vert_count {
                        let weight_vars = input.read_u32::<LittleEndian>()?;
                        let weight_2 = (((weight_vars & 0x3FF) as f32 / 1023.0) / 8.0)
                            + ((weight_vars >> 30) as f32 / 8.0);
                        let weight_3 = (((weight_vars >> 10) & 0x3FF) as f32 / 1023.0) / 3.0;
                        let weight_4 = (((weight_vars >> 20) & 0x3FF) as f32 / 1023.0) / 4.0;
                        let weight_1 = 1.0 - weight_2 - weight_3 - weight_4;
                        let vector = Vector4 {
                            x: weight_1,
                            y: weight_2,
                            z: weight_3,
                            w: weight_4,
                        };
                        weights.push(vector);
                    }
                }
                val => {
                    return Err(anyhow!("unknown weights format {}", val));
                }
            }
        }

        if has_bones > 0 {
            log::debug!(
                "Bone IDs start = {:#X}, bones_format = {}",
                input.stream_position()?,
                bones_format
            );
            match bones_format {
                33 => {
                    for _ in 0..vert_count {
                        let bone_info = BoneInfo::parse(&mut input)?;
                        bones_infos.push(bone_info);
                    }
                }
                val => return Err(anyhow!("unknown bones format {}", val)),
            }
        }

        let mut normals = Vec::new();
        if has_normals > 0 {
            log::debug!(
                "Normals start = {:#X}, normals_format = {}",
                input.stream_position()?,
                normals_format
            );
            match normals_format {
                38 => {
                    for _ in 0..vert_count {
                        // TODO: this might be wrong. Maybe there are two u16 stored in the four bytes
                        // and the normal data is recalculated in the shader?
                        let normal = parse_normal_from_i8(&mut input)?;
                        normals.push(normal);
                    }
                }
                26 => {
                    for _ in 0..vert_count {
                        // TODO: same issue as parse_normal_from_i8
                        let normal = parse_normal_from_i16(&mut input)?;
                        normals.push(normal);
                    }
                }
                val => return Err(anyhow!("unknown normals format {}", val)),
            }
        }

        if has_tangents > 0 {
            log::debug!("Tangents(?) start = {:#X}", input.stream_position()?);
            match tangents_format {
                38 => {
                    for _ in 0..vert_count {
                        // skip for now
                        let _1 = (input.read_i8()? as f32) / 127.0;
                        let _2 = (input.read_i8()? as f32) / 127.0;
                        let _3 = (input.read_i8()? as f32) / 127.0;
                        let _4 = (input.read_i8()? as f32) / 127.0;
                    }
                }
                val => return Err(anyhow!("unknown tangents format {}", val)),
            }
        }

        if has_binormals > 0 {
            log::debug!("Binormals(?) start = {:#X}", input.stream_position()?);
            match binormals_format {
                38 => {
                    for _ in 0..vert_count {
                        // skip for now
                        let _1 = (input.read_i8()? as f32) / 127.0;
                        let _2 = (input.read_i8()? as f32) / 127.0;
                        let _3 = (input.read_i8()? as f32) / 127.0;
                        let _4 = (input.read_i8()? as f32) / 127.0;
                    }
                }
                val => return Err(anyhow!("unknown binormals format {}", val)),
            }
        }

        let mut uv5_option = None;
        if has_uv5 > 0 {
            log::debug!(
                "UVs 5 start = {:#X} type = {}",
                input.stream_position()?,
                uv5_format
            );
            let uv5 = parse_uv_list(&mut input, vert_count, uv5_format, uv_clamps[4].as_ref())?;
            uv5_option = Some(uv5);
        }

        let mut uv6_option = None;
        if has_uv6 > 0 {
            log::debug!(
                "UVs 6 start = {:#X} type = {}",
                input.stream_position()?,
                uv6_format
            );
            let uv6 = parse_uv_list(&mut input, vert_count, uv6_format, uv_clamps[5].as_ref())?;
            uv6_option = Some(uv6);
        }

        if has_colors > 0 {
            log::debug!("Colors start = {:#X}", input.stream_position()?);
            match colors_format {
                33 | 39 => {
                    for _ in 0..vert_count {
                        let _r = input.read_u8()?;
                        let _g = input.read_u8()?;
                        let _b = input.read_u8()?;
                        let _a = input.read_u8()?;
                    }
                }
                val => return Err(anyhow!("unknown colors format {}", val)),
            }
        }

        if has_colors2 > 0 {
            log::debug!("Colors2 start = {:#X}", input.stream_position()?);
            match colors2_format {
                33 | 39 => {
                    for _ in 0..vert_count {
                        let _r = input.read_u8()?;
                        let _g = input.read_u8()?;
                        let _b = input.read_u8()?;
                        let _a = input.read_u8()?;
                    }
                }
                val => return Err(anyhow!("unknown colors2 format {}", val)),
            }
        }

        let mut uv = Vec::new();
        if has_uv1 > 0 {
            log::debug!(
                "UVs 1 start = {:#X} type = {}",
                input.stream_position()?,
                uv1_format
            );
            let uv1 = parse_uv_list(&mut input, vert_count, uv1_format, uv_clamps[0].as_ref())?;
            if uv1.len() > 0 {
                uv.push(uv1);
            }
        }

        if has_uv2 > 0 {
            log::debug!(
                "UVs 2 start = {:#X} type = {}",
                input.stream_position()?,
                uv2_format
            );
            let uv2 = parse_uv_list(&mut input, vert_count, uv2_format, uv_clamps[1].as_ref())?;
            if uv2.len() > 0 {
                uv.push(uv2);
            }
        }

        if has_uv3 > 0 {
            log::debug!(
                "UVs 3 start = {:#X} type = {}",
                input.stream_position()?,
                uv3_format
            );
            let uv3 = parse_uv_list(&mut input, vert_count, uv3_format, uv_clamps[2].as_ref())?;
            if uv3.len() > 0 {
                uv.push(uv3);
            }
        }

        if has_uv4 > 0 {
            log::debug!(
                "UVs 4 start = {:#X} type = {}",
                input.stream_position()?,
                uv4_format
            );
            let uv4 = parse_uv_list(&mut input, vert_count, uv4_format, uv_clamps[3].as_ref())?;
            if uv4.len() > 0 {
                uv.push(uv4);
            }
        }

        if let Some(uv5) = uv5_option {
            if uv5.len() > 0 {
                uv.push(uv5);
            }
        }
        if let Some(uv6) = uv6_option {
            if uv6.len() > 0 {
                uv.push(uv6);
            }
        }

        log::debug!("End of file = {:#X}", input.stream_position()?);

        // transform the indices of the bone_info via the provided bone IDs
        for bone_info in bones_infos {
            bones.push([
                bone_ids[bone_info.bone_1 as usize],
                bone_ids[bone_info.bone_2 as usize],
                bone_ids[bone_info.bone_3 as usize],
                bone_ids[bone_info.bone_4 as usize],
            ]);
        }

        Ok(Self {
            positions,
            uv,
            normals,
            faces,
            bones,
            weights,
        })
    }
}

fn parse_uv_list<T: Read>(
    mut input: T,
    vert_count: u32,
    uv_format: u32,
    uv_clamps: Option<&UVClamps>,
) -> Result<Vec<UV>> {
    let mut uvs = Vec::new();
    for _ in 0..vert_count {
        let uv = match uv_format {
            3 => UV::parse_f32(&mut input)?,
            24 => UV::parse_i16(&mut input, uv_clamps.unwrap_or(&UVClamps::default()))?,
            25 => UV::parse_u16(&mut input, uv_clamps.unwrap_or(&UVClamps::default()))?,
            val => return Err(anyhow!("unknown uv format {}", val)),
        };
        uvs.push(uv);
    }
    Ok(uvs)
}

/// Parses a position with bones information attached.
fn parse_position_with_bones<T: Read + Seek>(mut input: T) -> Result<(Vector3<f32>, BoneInfo)> {
    let vector = parse_vec3_f32(&mut input)?;
    let bone_info = BoneInfo::parse(&mut input)?;
    input.seek(SeekFrom::Current(0x08))?;
    Ok((vector, bone_info))
}

/// Parses a Normal from four i8
fn parse_normal_from_i8<T: Read>(mut input: T) -> Result<Vector3<f32>> {
    let x = (input.read_i8()? as f32) / 127.0;
    let y = (input.read_i8()? as f32) / 127.0;
    let z = (input.read_i8()? as f32) / 127.0;
    let _q = (input.read_i8()? as f32) / 127.0;
    let vector = Vector3 { x, y, z };
    // due to the bad accuracy of i8, re-normalize values
    Ok(vector.normalize())
}

/// Parses a Normal from four u16 values
fn parse_normal_from_i16<T: Read>(mut input: T) -> Result<Vector3<f32>> {
    let x = (input.read_i16::<LittleEndian>()? as f32) / 32767.0;
    let y = (input.read_i16::<LittleEndian>()? as f32) / 32767.0;
    let z = (input.read_i16::<LittleEndian>()? as f32) / 32767.0;
    let _q = (input.read_i16::<LittleEndian>()? as f32) / 32767.0;
    let vector = Vector3 { x, y, z };
    // due to the bad accuracy of i16, re-normalize values
    Ok(vector.normalize())
}

#[derive(Debug)]
pub struct UV {
    pub u: f32,
    pub v: f32,
}

impl UV {
    /// Parses UV-coordinates as two float32
    fn parse_f32<T: Read>(mut input: T) -> Result<Self> {
        let u = input.read_f32::<LittleEndian>()?;
        let v = input.read_f32::<LittleEndian>()?;
        Ok(Self { u, v })
    }

    /// Parses UV-coordinates as two i16
    fn parse_i16<T: Read>(mut input: T, uv_clamps: &UVClamps) -> Result<Self> {
        let u = input.read_i16::<LittleEndian>()?;
        let u = ((u as f32 / 32767.0) * uv_clamps.multiplier.u) + uv_clamps.start.u;
        let v = input.read_i16::<LittleEndian>()?;
        let v = ((v as f32 / 32767.0) * uv_clamps.multiplier.v) + uv_clamps.start.v;
        Ok(Self { u, v })
    }

    /// Parses UV-coordinates as two u16
    fn parse_u16<T: Read>(mut input: T, uv_clamps: &UVClamps) -> Result<Self> {
        let u = input.read_u16::<LittleEndian>()?;
        let u = ((u as f32 / 65535.0) * uv_clamps.multiplier.u) + uv_clamps.start.u;
        let v = input.read_u16::<LittleEndian>()?;
        let v = ((v as f32 / 65535.0) * uv_clamps.multiplier.v) + uv_clamps.start.v;
        Ok(Self { u, v })
    }
}

#[derive(Debug)]
struct BoneInfo {
    pub bone_1: u8,
    pub bone_2: u8,
    pub bone_3: u8,
    pub bone_4: u8,
}

impl BoneInfo {
    fn parse<T: Read>(mut input: T) -> Result<Self> {
        let bone_1 = input.read_u8()?;
        let bone_2 = input.read_u8()?;
        let bone_3 = input.read_u8()?;
        let bone_4 = input.read_u8()?;
        Ok(Self {
            bone_1,
            bone_2,
            bone_3,
            bone_4,
        })
    }
}

pub type BoneReference = [u64; 4];

#[derive(Debug, Clone, Copy)]
pub struct Face {
    pub a: u16,
    pub b: u16,
    pub c: u16,
}

impl Face {
    fn parse<T: Read>(mut input: T) -> Result<Self> {
        let a = input.read_u16::<LittleEndian>()?;
        let b = input.read_u16::<LittleEndian>()?;
        let c = input.read_u16::<LittleEndian>()?;
        Ok(Self { a, b, c })
    }
}

#[derive(Debug)]
pub struct ModelClamps {
    pub mesh_multiplier: Vector3<f32>,
    pub mesh_min: Vector3<f32>,
    pub orientation: ModelOrientation,
}

#[derive(Debug)]
pub enum ModelOrientation {
    Q,
    X,
    Y,
    Z,
}

#[derive(Debug)]
pub struct UVClamps {
    pub multiplier: UV,
    pub start: UV,
}

impl Default for UVClamps {
    fn default() -> Self {
        Self {
            multiplier: UV { u: 1.0, v: 1.0 },
            start: UV { u: 0.0, v: 0.0 },
        }
    }
}

pub type UVLayerClamps = [Option<UVClamps>; 6];
