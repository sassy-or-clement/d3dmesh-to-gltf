use std::{collections::HashMap, io::Write};

use anyhow::{anyhow, Context, Result};
use byteorder::WriteBytesExt;

use crate::skeleton::Skeleton;

use super::{writer::WriteTo, Material, WriterWithCounter};

/// Stores all relevant information for a rigged object.
/// This includes a skeleton (i.e. a number of joints (aka bones)) and one or more meshes
/// that reference this skeleton.
pub struct RiggedObject<W: Write> {
    root_object_name: Option<String>,
    dst: WriterWithCounter<W>,
    meshes: Vec<gltf_json::Mesh>,
    materials: Vec<gltf_json::Material>,
    nodes: Vec<gltf_json::Node>,
    root_children: Vec<gltf_json::Index<gltf_json::Node>>,
    buffer_views: Vec<gltf_json::buffer::View>,
    accessors: Vec<gltf_json::Accessor>,
    images: Vec<gltf_json::Image>,
    textures: Vec<gltf_json::Texture>,
    skins: Vec<gltf_json::Skin>,
}

/// Holds references to the base data for one mesh.
/// The internal references are indices of the accessors in the glTF file.
#[derive(Debug)]
pub struct BaseDataReference {
    positions: u32,
    normals: Option<u32>,
    uv: Vec<u32>,
    weights: Option<u32>,
    joints: Option<u32>,
}

/// Holds references to the materials for one mesh.
#[derive(Debug)]
pub struct MaterialReference {
    accessor_indices: Vec<u32>,
}

/// A MeshSet holds the information for multiple meshes that are in some way relevant to each other.
/// E.g. from the same .d3dmesh file.
#[derive(Debug)]
pub struct MeshSet<'a, D: WriteTo> {
    pub name: Option<String>,
    pub indices: &'a [D],
    pub uv_layer: Option<u32>,
    pub material_index: u32,
    pub skin_index: Option<u32>,
    pub base_data_reference: &'a BaseDataReference,
    pub material_reference: &'a MaterialReference,
}

impl<W> RiggedObject<W>
where
    W: Write,
{
    pub fn new(dst: W, root_name: Option<String>) -> Self {
        Self {
            root_object_name: root_name,
            dst: WriterWithCounter::new(dst),
            meshes: Vec::new(),
            materials: Vec::new(),
            nodes: Vec::new(),
            root_children: Vec::new(),
            buffer_views: Vec::new(),
            accessors: Vec::new(),
            images: Vec::new(),
            textures: Vec::new(),
            skins: Vec::new(),
        }
    }

    /// Adds one or more meshes with references to previously added materials and positions.
    /// The group of meshes gets its own parent node under the scene wide root node.
    /// I.e. root_node -> mesh_set_node_1, mesh_set_node_2, ...
    pub fn add_mesh_sets<D: WriteTo>(&mut self, mesh_sets: &[MeshSet<D>]) -> Result<()> {
        let mut children = Vec::new();
        for (i, mesh_set) in mesh_sets.iter().enumerate() {
            let name = if let Some(name) = &mesh_set.name {
                // Note: special case, if theres only one mesh_set, just use its unmodified name
                if mesh_sets.len() == 1 {
                    Some(name.clone())
                } else {
                    Some(format!("{}_mesh{}", name, i))
                }
            } else {
                None
            };

            let mesh_index = self
                .add_mesh_set(&mesh_set, i)
                .context("could not add mesh set")?;

            let skin_index = if let Some(skin_index) = mesh_set.skin_index {
                Some(gltf_json::Index::new(skin_index))
            } else {
                None
            };

            self.nodes.push(gltf_json::Node {
                camera: None,
                children: None,
                extensions: None,
                extras: gltf_json::Extras::default(),
                matrix: None,
                mesh: Some(gltf_json::Index::new(mesh_index)),
                name,
                rotation: None,
                scale: None,
                translation: None,
                skin: skin_index,
                weights: None,
            });
            let node_index = (self.nodes.len() - 1) as u32;
            children.push(gltf_json::Index::new(node_index));
        }

        self.root_children.extend(children);

        Ok(())
    }

    /// Adds one mesh with the references to the previously generated materials and positions.
    /// Returns the index to the mesh and DOES NOT create new nodes!
    fn add_mesh_set<D: WriteTo>(&mut self, mesh_set: &MeshSet<D>, index: usize) -> Result<u32> {
        let mut mesh_primitive_attributes: HashMap<
            gltf_json::validation::Checked<gltf_json::mesh::Semantic>,
            gltf_json::Index<gltf_json::Accessor>,
        > = HashMap::new();

        // Positions
        mesh_primitive_attributes.insert(
            gltf_json::validation::Checked::Valid(gltf_json::mesh::Semantic::Positions),
            gltf_json::Index::new(mesh_set.base_data_reference.positions),
        );

        // Faces / Indices
        let indices_accessor_index = if mesh_set.indices.len() > 0 {
            self.write_buffer_view_and_accessor(mesh_set.indices)?
        } else {
            panic!("indices are not allowed to be of length = 0")
        };

        // Normals
        if let Some(normals_accessor_index) = mesh_set.base_data_reference.normals {
            mesh_primitive_attributes.insert(
                gltf_json::validation::Checked::Valid(gltf_json::mesh::Semantic::Normals),
                gltf_json::Index::new(normals_accessor_index),
            );
        }

        // UV Layer
        if let Some(uv_layer) = mesh_set.uv_layer {
            if uv_layer >= mesh_set.base_data_reference.uv.len() as u32 {
                return Err(anyhow!(
                    "invalid uv layer {} of {}",
                    uv_layer,
                    mesh_set.base_data_reference.uv.len()
                ));
            } else {
                mesh_primitive_attributes.insert(
                    // Note: one mesh should only have one uv layer
                    // TODO add all other available uv layers?
                    gltf_json::validation::Checked::Valid(gltf_json::mesh::Semantic::TexCoords(0)),
                    gltf_json::Index::new(mesh_set.base_data_reference.uv[uv_layer as usize]),
                );
            }
        }

        // Weights
        for (i, weights_accessor_index) in mesh_set.base_data_reference.weights.iter().enumerate() {
            mesh_primitive_attributes.insert(
                gltf_json::validation::Checked::Valid(gltf_json::mesh::Semantic::Weights(i as u32)),
                gltf_json::Index::new(*weights_accessor_index),
            );
        }

        // Joints
        for (i, joints_accessor_index) in mesh_set.base_data_reference.joints.iter().enumerate() {
            mesh_primitive_attributes.insert(
                gltf_json::validation::Checked::Valid(gltf_json::mesh::Semantic::Joints(i as u32)),
                gltf_json::Index::new(*joints_accessor_index),
            );
        }

        // Note: a bit hacky, but overwrite the name of the mesh_set, so that the name of the mesh
        // is the same as the one of the node (see add_mesh_sets)
        let name = if let Some(name) = &mesh_set.name {
            Some(format!("{}_mesh{}", name, index))
        } else {
            None
        };

        self.meshes.push(gltf_json::Mesh {
            extensions: None,
            extras: gltf_json::Extras::default(),
            name,
            primitives: vec![gltf_json::mesh::Primitive {
                attributes: mesh_primitive_attributes,
                extensions: None,
                extras: gltf_json::Extras::default(),
                indices: Some(gltf_json::Index::new(indices_accessor_index)),
                material: Some(gltf_json::Index::new(
                    mesh_set.material_reference.accessor_indices[mesh_set.material_index as usize],
                )),
                mode: gltf_json::validation::Checked::Valid(gltf_json::mesh::Mode::Triangles),
                targets: None,
            }],
            weights: None,
        });

        Ok((self.meshes.len() - 1) as u32)
    }

    /// Adds a positions buffer and optional additional information.
    /// The created indexing information is returned to be used in subsequent add_mesh calls
    /// that reference this base data.
    pub fn add_shared_base_data<D, E, F, G, H>(
        &mut self,
        positions: &[D],
        normals: Option<&[E]>,
        uv: Option<&[Vec<F>]>,
        weights: Option<&[G]>,
        joints: Option<&[H]>,
    ) -> Result<BaseDataReference>
    where
        D: WriteTo,
        E: WriteTo,
        F: WriteTo,
        G: WriteTo,
        H: WriteTo,
    {
        let positions_index = self
            .write_buffer_view_and_accessor(positions)
            .context("could not write positions data")?;
        let mut reference = BaseDataReference {
            positions: positions_index,
            normals: None,
            uv: Vec::new(),
            weights: None,
            joints: None,
        };

        if let Some(normals) = normals {
            let index = self
                .write_buffer_view_and_accessor(normals)
                .context("could not write normal data")?;
            reference.normals = Some(index);
        }

        if let Some(uv) = uv {
            let mut uv_indices = Vec::new();
            // Only add one uv-layer for now
            // TODO more than one uv-layer? But what uses a second uv-layer?
            if uv.len() > 0 {
                let index = self
                    .write_buffer_view_and_accessor(&uv[0])
                    .context("could not write uv data")?;
                uv_indices.push(index);
            }
            reference.uv = uv_indices;
        }

        if let Some(weights) = weights {
            let index = self
                .write_buffer_view_and_accessor(weights)
                .context("could not write weights data")?;
            reference.weights = Some(index);
        }

        if let Some(joints) = joints {
            let index = self
                .write_buffer_view_and_accessor(joints)
                .context("could not write joints data")?;
            reference.joints = Some(index);
        }

        Ok(reference)
    }

    /// Adds a list of materials and returns the reference data to theses materials so that they can be used in
    /// subsequent calls to add_mesh.
    pub fn add_materials(&mut self, materials: &[Material]) -> MaterialReference {
        let mut accessor_indices = Vec::new();
        for material in materials {
            let mut gltf_material = gltf_json::Material::default();
            gltf_material.alpha_mode =
                gltf_json::validation::Checked::Valid(gltf_json::material::AlphaMode::Opaque);

            // Use the name of the diffuse texture as the material name (without the .png at the end)
            gltf_material.name = if let Some(diffuse_texture) = &material.diffuse_texture {
                Some(diffuse_texture[..diffuse_texture.len() - 4].to_string())
            } else {
                self.root_object_name.clone()
            };

            if let Some(diffuse_texture) = material.diffuse_texture.clone() {
                gltf_material.pbr_metallic_roughness.base_color_texture =
                    Some(self.set_general_texture(&diffuse_texture, 0));
            }
            if let Some(normal_texture) = material.normal_texture.clone() {
                gltf_material.normal_texture = Some(self.set_normal_texture(&normal_texture, 0));
            }
            if let Some(occlusion_roughness_metal_specular_texture) =
                material.occlusion_roughness_metal_specular_texture.clone()
            {
                gltf_material
                    .pbr_metallic_roughness
                    .metallic_roughness_texture =
                    Some(self.set_general_texture(&occlusion_roughness_metal_specular_texture, 0));
                // TODO KHR_materials_specular or KHR_materials_pbrSpecularGlossiness extension
            }
            self.materials.push(gltf_material);
            accessor_indices.push((self.materials.len() - 1) as u32);
        }
        MaterialReference { accessor_indices }
    }

    /// Adds a skeleton by adding a skin and the joints with their respective hierarchy.
    /// Returns the index of the skin, so that it can be applied for mesh-sets.
    pub fn add_skin(&mut self, skeleton: &Skeleton) -> Result<u32> {
        let (joints, root_bones) = {
            // to prevent recursion, there are two passes:
            // first: add all joints as nodes, but leave the hierarchy (i.e. the "children" field) empty
            // second: fill in the children fields via the created mapping in pass one
            let mut added_joints = Vec::new();
            let mut root_bones = Vec::new();

            // first pass
            // node_mapping gives the index for a joint-node (in the glTF structure; u32)
            // by asking for a joint-index (in the array from the skl-file; usize),
            // if the corresponding node was already created
            let mut node_joint_mapping: HashMap<usize, u32> = HashMap::new();
            for (i, joint) in skeleton.joints.iter().enumerate() {
                let mut rotation = gltf_json::scene::UnitQuaternion::default();
                rotation.0 = joint.rotation.into();
                self.nodes.push(gltf_json::Node {
                    camera: None,
                    children: None,
                    extensions: None,
                    extras: gltf_json::Extras::default(),
                    matrix: None,
                    mesh: None,
                    name: Some(joint.name.clone()),
                    rotation: Some(rotation),
                    scale: None,
                    translation: Some(joint.translation.into()),
                    skin: None,
                    weights: None,
                });
                let index = (self.nodes.len() - 1) as u32;
                added_joints.push(gltf_json::Index::new(index));
                node_joint_mapping.insert(i, index);

                // this is a root bone, if it does not have any parents
                if joint.parent == None {
                    root_bones.push(gltf_json::Index::new(index));
                }
            }

            // second pass
            for (joint_index, joint_node) in &node_joint_mapping {
                // find all children for the current joint; indices are the joint-indices from the skl file
                let mut children_indices = Vec::new();
                for (i, joint) in skeleton.joints.iter().enumerate() {
                    if joint.parent == Some(*joint_index as u32) {
                        children_indices.push(i);
                    }
                }
                // convert indices to node-indices via mapping
                let mut children = Vec::new();
                for unmapped_index in children_indices {
                    children.push(gltf_json::Index::new(
                        *(node_joint_mapping.get(&unmapped_index).unwrap()),
                    ));
                }

                // apply the generated children-list
                if children.len() > 0 {
                    // Note: applying Some(vec![]) (i.e. Some with vector of length = 0)
                    // is discouraged in the glTF spec
                    self.nodes[*joint_node as usize].children = Some(children);
                }
            }

            (added_joints, root_bones)
        };
        let skeleton_index = if root_bones.len() == 1 {
            Some(root_bones[0])
        } else {
            None
        };
        self.root_children.extend(root_bones);

        let inverse_bind_matrices_index =
            self.write_buffer_view_and_accessor(&skeleton.inverse_bind_matrices)?;

        self.skins.push(gltf_json::Skin {
            extensions: None,
            extras: gltf_json::Extras::default(),
            inverse_bind_matrices: Some(gltf_json::Index::new(inverse_bind_matrices_index)),
            joints,
            name: self.root_object_name.clone(),
            skeleton: skeleton_index,
        });
        Ok((self.skins.len() - 1) as u32)
    }

    /// Creates and writes to the buffer, creates a buffer view and one accessor.
    /// The index of the accessor is returned.
    fn write_buffer_view_and_accessor<D: WriteTo>(&mut self, data: &[D]) -> Result<u32> {
        let (component_type, object_type) = D::get_types();

        // note: glTF requires the data to be aligned on the component types size
        // so add padding before adding the buffer
        let current_start = self.dst.get_bytes_written();
        // the overhang is the amount of bytes after a possible alignment index
        // e.g. if start is 10 and component size is 8, the overhang is 2
        let overhang = current_start % component_type.size();
        if overhang != 0 {
            // padding required, the current start in the buffer is not aligned
            let padding = component_type.size() - overhang;
            for _ in 0..padding {
                self.dst.write_u8(0)?;
            }
        }

        let data_start = self.dst.get_bytes_written();
        let mut data_count = 0;
        let mut data_bytes = 0;
        for entry in data {
            let (bytes_written, count_elements) = entry.write_to(&mut self.dst)?;
            data_bytes += bytes_written;
            data_count += count_elements;
        }

        self.buffer_views.push(gltf_json::buffer::View {
            buffer: gltf_json::Index::new(0),
            byte_length: data_bytes as u32,
            byte_offset: Some(data_start as u32),
            byte_stride: None,
            name: None,
            target: None,
            extensions: None,
            extras: gltf_json::Extras::default(),
        });

        let (min, max) = D::calculate_min_and_max(data);
        self.accessors.push(gltf_json::Accessor {
            buffer_view: Some(gltf_json::Index::new((self.buffer_views.len() - 1) as u32)),
            byte_offset: 0,
            count: data_count as u32,
            component_type: gltf_json::validation::Checked::Valid(
                gltf_json::accessor::GenericComponentType(component_type),
            ),
            extensions: None,
            extras: gltf_json::Extras::default(),
            type_: gltf_json::validation::Checked::Valid(object_type),
            min,
            max,
            name: None,
            normalized: false,
            sparse: None,
        });

        Ok((self.accessors.len() - 1) as u32)
    }

    /// Sets the given path for general textures (e.g. diffuse or specular).
    fn set_general_texture(&mut self, path: &str, tex_coord: u32) -> gltf_json::texture::Info {
        let index = self.add_image_and_texture(path);
        gltf_json::texture::Info {
            index: gltf_json::Index::new(index),
            tex_coord,
            extensions: None,
            extras: gltf_json::Extras::default(),
        }
    }

    /// Sets the given path as the normal map texture.
    fn set_normal_texture(
        &mut self,
        path: &str,
        tex_coord: u32,
    ) -> gltf_json::material::NormalTexture {
        let index = self.add_image_and_texture(path);
        gltf_json::material::NormalTexture {
            index: gltf_json::Index::new(index),
            scale: 1.0,
            tex_coord,
            extensions: None,
            extras: gltf_json::Extras::default(),
        }
    }

    /// Adds the path as image and texture, returning the texture index.
    fn add_image_and_texture(&mut self, path: &str) -> u32 {
        self.images.push(gltf_json::Image {
            buffer_view: None,
            mime_type: None,
            name: None,
            uri: Some(path.to_string()),
            extensions: None,
            extras: gltf_json::Extras::default(),
        });
        let image_index = (self.images.len() - 1) as u32;
        self.textures.push(gltf_json::Texture {
            name: None,
            sampler: None,
            source: gltf_json::Index::new(image_index),
            extensions: None,
            extras: gltf_json::Extras::default(),
        });
        (self.textures.len() - 1) as u32
    }

    /// Creates the glTF 2.0 JSON data and writes it to dst.
    /// The file_name_binary must be provided, which is the URI to the buffer file.
    /// Optionally provide the index of the armature/skin that should be used for the whole object.
    pub fn write_gltf_json<T: Write>(self, mut dst: T, file_name_binary: String) -> Result<()> {
        let gltf = gltf_json::root::Root {
            accessors: self.accessors,
            animations: Vec::new(),
            asset: gltf_json::Asset {
                version: "2.0".to_string(),
                ..gltf_json::Asset::default()
            },
            buffers: vec![gltf_json::Buffer {
                byte_length: self.dst.get_bytes_written() as u32,
                uri: Some(file_name_binary),
                name: None,
                extensions: None,
                extras: gltf_json::Extras::default(),
            }],
            buffer_views: self.buffer_views,
            scene: None,
            extensions: None,
            extras: gltf_json::Extras::default(),
            extensions_used: Vec::new(),
            extensions_required: Vec::new(),
            cameras: Vec::new(),
            images: self.images,
            materials: self.materials,
            meshes: self.meshes,
            nodes: self.nodes,
            samplers: Vec::new(),
            scenes: vec![gltf_json::Scene {
                extensions: None,
                extras: gltf_json::Extras::default(),
                name: None,
                nodes: self.root_children,
            }],
            skins: self.skins,
            textures: self.textures,
        };

        let json = gltf_json::serialize::to_string_pretty(&gltf)?;
        dst.write_all(json.as_bytes())
            .context("could not write glTF JSON data")
    }
}
