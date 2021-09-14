use std::io::{Read, Seek, SeekFrom};

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use cgmath::{Basis3, Matrix3, Matrix4, Quaternion, Transform, Vector3, Vector4};

use crate::{
    byte_reading::{parse_vec3_f32, parse_vec4_f32, VersionHeader},
    checksum_mapping::ChecksumMap,
};

#[derive(Debug)]
pub struct Skeleton {
    pub joints: Vec<Joint>,
    pub inverse_bind_matrices: Vec<Matrix4<f32>>,
}

impl Skeleton {
    pub fn parse<R: Read + Seek>(mut input: R, checksum_mapping: &ChecksumMap) -> Result<Self> {
        let version = VersionHeader::parse(&mut input)?;
        match version {
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
        let _file_size = input.read_u32::<LittleEndian>()?;
        let joint_count = input.read_u32::<LittleEndian>()?;
        let mut joints = Vec::new();
        for _ in 0..joint_count {
            let joint = Joint::parse(&mut input, checksum_mapping)?;
            joints.push(joint);
        }
        let inverse_bind_matrices = calculate_inverse_bind_matrices(&joints);
        assert_eq!(joints.len(), inverse_bind_matrices.len());
        Ok(Self {
            joints,
            inverse_bind_matrices,
        })
    }
}

#[derive(Debug)]
pub struct Joint {
    pub parent: Option<u32>,
    pub translation: Vector3<f32>,
    pub rotation: Vector4<f32>,
    pub name: String,
    pub id: u64,
}

impl Joint {
    fn parse<R: Read + Seek>(mut input: R, checksum_mapping: &ChecksumMap) -> Result<Self> {
        let bone_checksum = input.read_u64::<LittleEndian>()?;
        // get name from bone by checksum mapping or default to hex string
        let name = checksum_mapping
            .get_mapping(bone_checksum)
            .unwrap_or(format!("{:016x}", bone_checksum));
        let _bone_parent_checksum = input.read_u64::<LittleEndian>()?;
        let bone_parent = input.read_u32::<LittleEndian>()?;
        // bone_parent may be "unset" (i.e. no parent), which is indicated by 0xFFFFFFFF (?)
        let parent = if bone_parent > 1000000 {
            None
        } else {
            Some(bone_parent)
        };
        // skip unknowns
        input.seek(SeekFrom::Current(0x0C))?;

        let translation = parse_vec3_f32(&mut input)?;
        let rotation = parse_vec4_f32(&mut input)?;

        // skip unknowns
        input.seek(SeekFrom::Current(0x48))?;

        // skip variable length unknowns
        let amount = input.read_u32::<LittleEndian>()?;
        for _ in 0..amount {
            input.seek(SeekFrom::Current(0x0C))?;
        }

        input.seek(SeekFrom::Current(0x04))?;

        // skip variable length unknowns
        let amount = input.read_u32::<LittleEndian>()?;
        for _ in 0..amount {
            input.seek(SeekFrom::Current(0x0C))?;
        }

        input.seek(SeekFrom::Current(0x20))?;

        Ok(Self {
            parent,
            translation,
            rotation,
            name,
            id: bone_checksum,
        })
    }
}

/// Creates a list of inverse binding matrices for the given list containing a bone hierarchy.
fn calculate_inverse_bind_matrices(joints: &[Joint]) -> Vec<Matrix4<f32>> {
    fn inverse_bind_matrix_for_joint(joints: &[Joint], index: usize) -> Matrix4<f32> {
        let joint = &joints[index];
        // generate TRS transformation matrix for this joint
        let rotation_quaternion: [f32; 4] = joint.rotation.into();
        let rotation_quaternion: Quaternion<f32> = rotation_quaternion.into();
        let rotation: Matrix3<f32> = Basis3::from_quaternion(&rotation_quaternion).into();
        // 3x3 rotation matrix into 4x4 matrix
        let matrix = Matrix4::from_cols(
            Vector4::new(rotation.x.x, rotation.x.y, rotation.x.z, 0.0),
            Vector4::new(rotation.y.x, rotation.y.y, rotation.y.z, 0.0),
            Vector4::new(rotation.z.x, rotation.z.y, rotation.z.z, 0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );
        let matrix = Matrix4::from_translation(joint.translation) * matrix;

        // see https://computergraphics.stackexchange.com/questions/7603/confusion-about-how-inverse-bind-pose-is-actually-calculated-and-used
        // for how to generate inverse bind matrices
        let mut matrix = matrix.inverse_transform().unwrap();
        // Note: sometimes there are issues with the 15th element (column 4; row 4) in the matrix
        // It should be 1.0, but sometimes the inverse-calculation results in something like 0.9999998807907104
        // Therefore it is set manually to 1.0, if the difference to 1.0 is very small
        if f32::abs(matrix.w.w - 1.0) < 0.0001 {
            matrix.w.w = 1.0;
        }

        if let Some(parent) = joint.parent {
            matrix * inverse_bind_matrix_for_joint(joints, parent as usize)
        } else {
            matrix
        }
    }

    let mut matrices = Vec::new();
    for index in 0..joints.len() {
        matrices.push(inverse_bind_matrix_for_joint(joints, index));
    }
    matrices
}
