use std::io::Write;

use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use cgmath::{Matrix4, Vector3, Vector4};

use crate::d3dmesh::mesh::{Face, UV};

use super::JointInfo;

pub trait WriteTo: Sized {
    /// Returns the bytes written (tuple.0) and the count of elements that were written (tuple.1).
    fn write_to<W: Write>(&self, dst: W) -> Result<(u64, u32)>;

    /// Returns the types of the values that are writable.
    fn get_types() -> (
        gltf_json::accessor::ComponentType,
        gltf_json::accessor::Type,
    );

    /// Calculates per-component minimum and maximum values for a given list of items of this type.
    fn calculate_min_and_max(data: &[Self])
        -> (Option<gltf_json::Value>, Option<gltf_json::Value>);
}

impl WriteTo for JointInfo {
    fn write_to<W: Write>(&self, mut dst: W) -> Result<(u64, u32)> {
        let mut written = 0;

        for joint in self {
            dst.write_u8(*joint)?;
            written += 1;
        }

        Ok((written, 1))
    }

    fn get_types() -> (
        gltf_json::accessor::ComponentType,
        gltf_json::accessor::Type,
    ) {
        (
            gltf_json::accessor::ComponentType::U8,
            gltf_json::accessor::Type::Vec4,
        )
    }

    fn calculate_min_and_max(
        _data: &[Self],
    ) -> (Option<gltf_json::Value>, Option<gltf_json::Value>) {
        (None, None)
    }
}

impl WriteTo for UV {
    fn write_to<W: Write>(&self, mut dst: W) -> Result<(u64, u32)> {
        let mut written = 0;

        dst.write_f32::<LittleEndian>(self.u)?;
        written += 4;
        dst.write_f32::<LittleEndian>(self.v)?;
        written += 4;

        Ok((written, 1))
    }

    fn get_types() -> (
        gltf_json::accessor::ComponentType,
        gltf_json::accessor::Type,
    ) {
        (
            gltf_json::accessor::ComponentType::F32,
            gltf_json::accessor::Type::Vec2,
        )
    }

    fn calculate_min_and_max(
        data: &[Self],
    ) -> (Option<gltf_json::Value>, Option<gltf_json::Value>) {
        if data.len() <= 0 {
            (None, None)
        } else {
            let mut min_u = f32::INFINITY;
            let mut min_v = f32::INFINITY;
            let mut max_u = f32::NEG_INFINITY;
            let mut max_v = f32::NEG_INFINITY;
            for item in data {
                min_u = f32::min(min_u, item.u);
                min_v = f32::min(min_v, item.v);
                max_u = f32::max(max_u, item.u);
                max_v = f32::max(max_v, item.v);
            }
            // return as array with two entries, since this is a Vec2
            (
                Some(gltf_json::Value::Array(vec![
                    gltf_json::serialize::to_value(min_u).unwrap(),
                    gltf_json::serialize::to_value(min_v).unwrap(),
                ])),
                Some(gltf_json::Value::Array(vec![
                    gltf_json::serialize::to_value(max_u).unwrap(),
                    gltf_json::serialize::to_value(max_v).unwrap(),
                ])),
            )
        }
    }
}

impl WriteTo for Vector3<f32> {
    fn write_to<W: Write>(&self, mut dst: W) -> Result<(u64, u32)> {
        let mut written = 0;

        dst.write_f32::<LittleEndian>(self.x)?;
        written += 4;
        dst.write_f32::<LittleEndian>(self.y)?;
        written += 4;
        dst.write_f32::<LittleEndian>(self.z)?;
        written += 4;

        Ok((written, 1))
    }

    fn get_types() -> (
        gltf_json::accessor::ComponentType,
        gltf_json::accessor::Type,
    ) {
        (
            gltf_json::accessor::ComponentType::F32,
            gltf_json::accessor::Type::Vec3,
        )
    }

    fn calculate_min_and_max(
        data: &[Self],
    ) -> (Option<gltf_json::Value>, Option<gltf_json::Value>) {
        if data.len() <= 0 {
            (None, None)
        } else {
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut min_z = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            let mut max_z = f32::NEG_INFINITY;
            for item in data {
                min_x = f32::min(min_x, item.x);
                min_y = f32::min(min_y, item.y);
                min_z = f32::min(min_z, item.z);
                max_x = f32::max(max_x, item.x);
                max_y = f32::max(max_y, item.y);
                max_z = f32::max(max_z, item.z);
            }
            // return as array with three entries, since this is a Vec3
            (
                Some(gltf_json::Value::Array(vec![
                    gltf_json::serialize::to_value(min_x).unwrap(),
                    gltf_json::serialize::to_value(min_y).unwrap(),
                    gltf_json::serialize::to_value(min_z).unwrap(),
                ])),
                Some(gltf_json::Value::Array(vec![
                    gltf_json::serialize::to_value(max_x).unwrap(),
                    gltf_json::serialize::to_value(max_y).unwrap(),
                    gltf_json::serialize::to_value(max_z).unwrap(),
                ])),
            )
        }
    }
}

impl WriteTo for Vector4<f32> {
    fn write_to<W: Write>(&self, mut dst: W) -> Result<(u64, u32)> {
        let mut written = 0;

        dst.write_f32::<LittleEndian>(self.x)?;
        written += 4;
        dst.write_f32::<LittleEndian>(self.y)?;
        written += 4;
        dst.write_f32::<LittleEndian>(self.z)?;
        written += 4;
        dst.write_f32::<LittleEndian>(self.w)?;
        written += 4;

        Ok((written, 1))
    }

    fn get_types() -> (
        gltf_json::accessor::ComponentType,
        gltf_json::accessor::Type,
    ) {
        (
            gltf_json::accessor::ComponentType::F32,
            gltf_json::accessor::Type::Vec4,
        )
    }

    fn calculate_min_and_max(
        data: &[Self],
    ) -> (Option<gltf_json::Value>, Option<gltf_json::Value>) {
        if data.len() <= 0 {
            (None, None)
        } else {
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut min_z = f32::INFINITY;
            let mut min_w = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            let mut max_z = f32::NEG_INFINITY;
            let mut max_w = f32::NEG_INFINITY;
            for item in data {
                min_x = f32::min(min_x, item.x);
                min_y = f32::min(min_y, item.y);
                min_z = f32::min(min_z, item.z);
                min_w = f32::min(min_w, item.w);
                max_x = f32::max(max_x, item.x);
                max_y = f32::max(max_y, item.y);
                max_z = f32::max(max_z, item.z);
                max_w = f32::max(max_w, item.w);
            }
            // return as array with three entries, since this is a Vec3
            (
                Some(gltf_json::Value::Array(vec![
                    gltf_json::serialize::to_value(min_x).unwrap(),
                    gltf_json::serialize::to_value(min_y).unwrap(),
                    gltf_json::serialize::to_value(min_z).unwrap(),
                    gltf_json::serialize::to_value(min_w).unwrap(),
                ])),
                Some(gltf_json::Value::Array(vec![
                    gltf_json::serialize::to_value(max_x).unwrap(),
                    gltf_json::serialize::to_value(max_y).unwrap(),
                    gltf_json::serialize::to_value(max_z).unwrap(),
                    gltf_json::serialize::to_value(max_w).unwrap(),
                ])),
            )
        }
    }
}

impl WriteTo for Matrix4<f32> {
    fn write_to<W: Write>(&self, mut dst: W) -> Result<(u64, u32)> {
        let mut written = 0;

        let (added_written, _count) = self.x.write_to(&mut dst)?;
        written += added_written;
        let (added_written, _count) = self.y.write_to(&mut dst)?;
        written += added_written;
        let (added_written, _count) = self.z.write_to(&mut dst)?;
        written += added_written;
        let (added_written, _count) = self.w.write_to(&mut dst)?;
        written += added_written;

        Ok((written, 1))
    }

    fn get_types() -> (
        gltf_json::accessor::ComponentType,
        gltf_json::accessor::Type,
    ) {
        (
            gltf_json::accessor::ComponentType::F32,
            gltf_json::accessor::Type::Mat4,
        )
    }

    fn calculate_min_and_max(
        _data: &[Self],
    ) -> (Option<gltf_json::Value>, Option<gltf_json::Value>) {
        (None, None)
    }
}

impl WriteTo for Face {
    fn write_to<W: Write>(&self, mut dst: W) -> Result<(u64, u32)> {
        let mut written = 0;

        dst.write_u16::<LittleEndian>(self.a)?;
        written += 2;
        dst.write_u16::<LittleEndian>(self.b)?;
        written += 2;
        dst.write_u16::<LittleEndian>(self.c)?;
        written += 2;

        // Note: one face isn't actually a vector3, but three scalar values
        Ok((written, 3))
    }

    fn get_types() -> (
        gltf_json::accessor::ComponentType,
        gltf_json::accessor::Type,
    ) {
        (
            gltf_json::accessor::ComponentType::U16,
            gltf_json::accessor::Type::Scalar,
        )
    }

    fn calculate_min_and_max(
        _data: &[Self],
    ) -> (Option<gltf_json::Value>, Option<gltf_json::Value>) {
        (None, None)
    }
}
