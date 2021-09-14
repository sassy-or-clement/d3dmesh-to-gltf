mod rigged_object;
mod writer;

use std::{convert::TryInto, io::Write, path::Path};

use anyhow::{anyhow, Context, Result};
use cgmath::{Vector3, Vector4};

use crate::{
    d3dmesh::{
        self,
        mesh::{BoneReference, Face},
        polygons::PolygonInfo,
        textures::{TextureMap, TextureType},
    },
    export::rigged_object::MeshSet,
    skeleton::Skeleton,
};

use self::rigged_object::RiggedObject;

struct WriterWithCounter<W: Write> {
    inner: W,
    bytes_written: usize,
}

impl<W> Write for WriterWithCounter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.bytes_written += written;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl<W> WriterWithCounter<W>
where
    W: Write,
{
    fn new(writer: W) -> Self {
        Self {
            inner: writer,
            bytes_written: 0,
        }
    }

    fn get_bytes_written(&self) -> usize {
        self.bytes_written
    }
}

/// Holds information about a typical PBR material with textures
pub struct Material {
    pub diffuse_texture: Option<String>,
    pub normal_texture: Option<String>,
    pub occlusion_roughness_metal_specular_texture: Option<String>,
}

/// JointInfo holds the indices for joints from a skeleton
pub type JointInfo = [u8; 4];

/// Writes the mesh to a binary file and returns the correct information for this buffer for glTF 2.0.
/// buffer_index is the index of the buffer information field for this binary file.
pub fn mesh_to_binary<W: Write>(
    dst_binary: W,
    file_name_binary: String,
    dst_json: W,
    texture_folder: &str,
    mesh: &d3dmesh::Data,
    name: Option<String>,
) -> Result<()> {
    // Note: a simple single object is just a rigged object without the rigging
    let mut single_object = RiggedObject::new(dst_binary, name.clone());

    let base_data_reference = single_object
        .add_shared_base_data::<Vector3<f32>, Vector3<f32>, d3dmesh::mesh::UV, Vector4<f32>,Vector4<f32>>(
            &mesh.mesh.positions,
            Some(&mesh.mesh.normals),
            Some(&mesh.mesh.uv),
            None,
            None,
        )?;

    let materials = convert_materials(texture_folder, &mesh.materials);
    let material_reference = single_object.add_materials(&materials);

    let separated_meshes = separate_mesh(&mesh.polygons, &mesh.mesh.faces);

    let mut mesh_sets = Vec::new();
    for separated_mesh in &separated_meshes {
        // TODO uv_layer might be the index or same as material index?
        mesh_sets.push(MeshSet {
            name: name.clone(),
            indices: &separated_mesh.faces,
            uv_layer: Some(0),
            material_index: separated_mesh.material_index,
            skin_index: None,
            base_data_reference: &base_data_reference,
            material_reference: &material_reference,
        });
    }
    single_object
        .add_mesh_sets(&mesh_sets)
        .context("could not add mesh sets")?;

    single_object.write_gltf_json(dst_json, file_name_binary)
}

/// Writes multiple meshes to a glTF file and its binary-file. All meshes get the skeleton assigned to them.
pub fn rigged_object_to_binary<W: Write>(
    dst_binary: W,
    file_name_binary: String,
    dst_json: W,
    texture_folder: &str,
    root_name: Option<String>,
    meshes: &[(String, d3dmesh::Data)],
    skeleton: &Skeleton,
) -> Result<()> {
    let mut rigged_object = RiggedObject::new(dst_binary, root_name);
    let skin_index = rigged_object.add_skin(skeleton)?;

    // add meshes
    for (mesh_name, mesh_data) in meshes {
        let joints = bone_ids_to_indices(skeleton, &mesh_data.mesh.bones)?;
        let base_data_reference = rigged_object.add_shared_base_data(
            &mesh_data.mesh.positions,
            Some(&mesh_data.mesh.normals),
            Some(&mesh_data.mesh.uv),
            Some(&mesh_data.mesh.weights),
            Some(&joints),
        )?;

        let materials = convert_materials(texture_folder, &mesh_data.materials);
        let material_reference = rigged_object.add_materials(&materials);

        let separated_meshes = separate_mesh(&mesh_data.polygons, &mesh_data.mesh.faces);

        let mut mesh_sets = Vec::new();
        for separated_mesh in &separated_meshes {
            // TODO uv_layer might be the index or same as material index?
            mesh_sets.push(MeshSet {
                name: Some(mesh_name.to_string()),
                indices: &separated_mesh.faces,
                uv_layer: Some(0),
                material_index: separated_mesh.material_index,
                skin_index: Some(skin_index),
                base_data_reference: &base_data_reference,
                material_reference: &material_reference,
            });
        }
        rigged_object
            .add_mesh_sets(&mesh_sets)
            .context("could not add mesh sets")?;
    }

    rigged_object.write_gltf_json(dst_json, file_name_binary)
}

/// Converts a given list with material information from d3dmesh files to the local glTF Material counterpart.
fn convert_materials(
    texture_folder: &str,
    materials: &[d3dmesh::materials::Material],
) -> Vec<Material> {
    let mut material_information_converted = Vec::new();
    for material in materials {
        let mut material_info = Material {
            diffuse_texture: None,
            normal_texture: None,
            occlusion_roughness_metal_specular_texture: None,
        };
        for texture in &material.textures {
            if texture.map == TextureMap::Map || texture.map == TextureMap::MapA {
                match texture.kind {
                    TextureType::Diffuse => {
                        material_info.diffuse_texture =
                            Some(texture_name_to_path(texture_folder, &texture.name));
                    }
                    TextureType::Normal => {
                        material_info.normal_texture =
                            Some(texture_name_to_path(texture_folder, &texture.name));
                    }
                    TextureType::Specular => {
                        material_info.occlusion_roughness_metal_specular_texture =
                            Some(texture_name_to_path(texture_folder, &texture.name));
                    }
                    _ => {}
                }
            }
        }
        material_information_converted.push(material_info);
    }
    material_information_converted
}

/// Uses a texture name (without any file extension) and returns a path with added png file extension as string.
fn texture_name_to_path(texture_folder: &str, texture_name: &str) -> String {
    // note: texture_path in glTF needs to be a URI. I.e. a/b is good a\b is bad
    let texture_with_extension = Path::new(texture_name)
        .with_extension("png")
        .to_str()
        .expect(&format!("invalid texture path {:?}", texture_name))
        .to_string();
    format!("{}/{}", texture_folder, texture_with_extension)
}

/// Transforms Bone IDs (CRC64 values) to indices by using a skeleton that includes the bone IDs
fn bone_ids_to_indices(
    skeleton: &Skeleton,
    bone_references: &[BoneReference],
) -> Result<Vec<JointInfo>> {
    let find_index = |id: u64| {
        for (index, joint) in skeleton.joints.iter().enumerate() {
            if joint.id == id {
                return Some(index);
            }
        }
        None
    };

    let mut joints = Vec::new();
    for bone_reference in bone_references {
        let mut joint_info = Vec::new();
        for reference in bone_reference {
            let joint = find_index(*reference).ok_or(anyhow!(
                "could not find index of bone referencing {}",
                reference
            ))?;
            joint_info.push(joint as u8);
        }
        let joint_info: &[u8] = &joint_info;
        let joint_info: [u8; 4] = joint_info.try_into().unwrap();
        joints.push(joint_info);
    }
    Ok(joints)
}

struct SeparatedMesh {
    faces: Vec<Face>,
    material_index: u32,
}

/// Separates a mesh via the polygon information into multiple meshes with distinct face-sets.
fn separate_mesh(polygons: &[PolygonInfo], faces: &[Face]) -> Vec<SeparatedMesh> {
    let mut separated_meshes = Vec::new();
    for poly_info in polygons {
        let range_start = poly_info.polygon_start as usize;
        let range_end = range_start + poly_info.polygon_count as usize;
        let separated_faces = faces[range_start..range_end].to_vec();
        separated_meshes.push(SeparatedMesh {
            faces: separated_faces,
            material_index: poly_info.mat_num,
        });
    }
    separated_meshes
}
