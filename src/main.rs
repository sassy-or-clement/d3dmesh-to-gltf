// Based on Telltale Games "Almost-All-In-One" model importer by Random Talking Bush

mod byte_reading;
mod checksum_mapping;
mod d3dmesh;
mod d3dtx;
mod export;
mod image_conversion;
mod logging;
mod runtime_config;
mod skeleton;

use std::{
    ffi::OsStr,
    fs::{self, File},
    io::Cursor,
    path::Path,
};

use anyhow::{anyhow, Context, Result};
use checksum_mapping::ChecksumMap;
use chrono::Local;
use rayon::iter::{ParallelBridge, ParallelIterator};
use runtime_config::Config;
use skeleton::Skeleton;

use crate::d3dmesh::textures::{TextureMap, TextureType};

fn main() -> Result<()> {
    // Note: all CRC64 checksums are actually CRC64_ECMA_182!

    // get command line flags
    let config = Config::new().context("could not parse command line flags")?;

    let input_folder = &config.input_folder;
    let output_folder = &config.output_folder;
    let texture_folder = &"textures".to_string();
    let texture_folder_absolute = Path::new(output_folder).join(texture_folder);
    // create output directory if necessary
    std::fs::create_dir_all(output_folder)?;
    std::fs::create_dir_all(texture_folder_absolute.clone())?;

    // logging setup
    let now = Local::now();
    let log_file_path =
        Path::new(&config.output_folder).join(now.format("%Y-%m-%d_%H-%M-%S.log").to_string());
    if config.verbose {
        logging::init(log::LevelFilter::Trace, log_file_path)
            .context("could not set logging level to verbose")?;
        // disable parallel working on verbose flag
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build_global()
            .context("could not set rayon number of threads")?;
    } else {
        logging::init(log::LevelFilter::Info, log_file_path)
            .context("could not set default logging level")?;
    }

    // static mapping table
    let checksum_mapping = ChecksumMap::new();

    if !config.disable_d3dmesh_conversion {
        log::info!("converting *.d3dmesh files...");
        // handle single-mesh conversion (i.e. one .d3dmesh -> one .gltf and one .bin)
        fs::read_dir(input_folder)
            .context("could not read input folder")?
            .par_bridge()
            .for_each(|entry| {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();

                        // filter for *.d3dmesh files
                        if path.extension() != Some(OsStr::new("d3dmesh")) {
                            return;
                        }

                        let err = handle_d3dmesh_file(
                            &path,
                            &config,
                            &checksum_mapping,
                            input_folder,
                            texture_folder,
                            &texture_folder_absolute,
                            output_folder,
                        );
                        if let Err(err) = err {
                            log::error!("Error: {}: {:?}", path.to_string_lossy(), err);
                        }
                    }
                    Err(err) => {
                        log::error!("unknown error for entry: {}", err);
                    }
                };
            });
    }

    if !config.disable_skl_conversion {
        log::info!("converting *.skl files...");
        // handle .skl (skeletons) files (i.e. create one .gltf and one .bin file for one .skl)
        // Note: creates redundant data from the .d3dmesh files, as these are also present in the
        // .bin-file for the skeleton
        fs::read_dir(input_folder)
            .context("could not read input folder")?
            .par_bridge()
            .for_each(|entry| {
                match entry {
                    Ok(entry) => {
                        let skeleton_path = entry.path();

                        // filter for *.skl files
                        if skeleton_path.extension() != Some(OsStr::new("skl")) {
                            return;
                        }

                        let err = handle_skl_file(
                            &skeleton_path,
                            &config,
                            &checksum_mapping,
                            input_folder,
                            texture_folder,
                            &texture_folder_absolute,
                            output_folder,
                        );
                        if let Err(err) = err {
                            log::error!("Error: {}: {:?}", skeleton_path.to_string_lossy(), err);
                        }
                    }
                    Err(err) => {
                        log::error!("unknown error for entry: {}", err);
                    }
                };
            });
    }

    Ok(())
}

/// handles a d3dmesh file by creating a corresponding glTF file for it.
fn handle_d3dmesh_file<P: AsRef<Path>>(
    path: P,
    config: &Config,
    checksum_mapping: &ChecksumMap,
    input_folder: &str,
    texture_folder: &str,
    texture_folder_absolute: &Path,
    output_folder: &str,
) -> Result<()> {
    let file = fs::read(&path).context("could not open d3dmesh file")?;
    let mut input = Cursor::new(file);

    let mesh =
        d3dmesh::Data::parse(&mut input, &checksum_mapping).context("could not parse mesh data")?;

    copy_textures(
        &config,
        input_folder,
        &texture_folder_absolute,
        &mesh.materials,
    )
    .context("could not copy textures from input to output")?;

    let mesh_name =
        get_file_name_from_path(path.as_ref()).context("could not get mesh file name")?;
    create_gltf(&mesh, output_folder, texture_folder, mesh_name)
        .context("could not create glTF 2.0 data")?;

    Ok(())
}

/// handles a skl file and reads d3dmesh files accordingly
fn handle_skl_file<P: AsRef<Path>>(
    skeleton_path: P,
    config: &Config,
    checksum_mapping: &ChecksumMap,
    input_folder: &str,
    texture_folder: &str,
    texture_folder_absolute: &Path,
    output_folder: &str,
) -> Result<()> {
    let skeleton_file_name = get_file_name_from_path(skeleton_path.as_ref())
        .context("could not get skeleton file name")?;

    let skeleton_file = fs::read(&skeleton_path).context("could not open skl file")?;
    let mut skeleton_input = Cursor::new(skeleton_file);

    let skeleton = Skeleton::parse(&mut skeleton_input, &checksum_mapping)
        .context("could not parse skeleton data")?;

    let mut meshes_using_skeleton = Vec::new();
    // open all potential meshes that use the skeleton
    // the name of the skl-file is used to filter for theses
    for entry in fs::read_dir(input_folder)? {
        let entry = entry?;
        let path = entry.path();

        // filter for *.d3dmesh files with the given prefix (prefix = skl-file name without extension)
        if path.extension() == Some(OsStr::new("d3dmesh"))
            && path
                .file_stem()
                .unwrap_or(OsStr::new(""))
                .to_str()
                .unwrap_or("")
                .starts_with(skeleton_file_name)
        {
            let mesh_file_name = get_file_name_from_path(&path)
                .context("could not get skeleton user mesh file name")?;
            let file = fs::read(&path).context("could not open d3dmesh file")?;
            let mut input = Cursor::new(file);

            let mesh = d3dmesh::Data::parse(&mut input, &checksum_mapping)
                .context("could not parse mesh data")?;

            // handle the textures of the mesh file
            copy_textures(
                &config,
                input_folder,
                &texture_folder_absolute,
                &mesh.materials,
            )
            .context("could not copy textures from input to output")?;

            meshes_using_skeleton.push((mesh_file_name.to_string(), mesh));
        }
    }

    create_rigged_gltf(
        &meshes_using_skeleton,
        output_folder,
        texture_folder,
        skeleton_file_name,
        &skeleton,
    )
    .context("could not create rigged glTF files")?;

    Ok(())
}

/// Note: texture_folder is relative to the output_folder.
/// E.g. output_folder = "output" and texture_folder = "textures" results in textures being in "output/textures".
fn create_gltf(
    mesh: &d3dmesh::Data,
    output_folder: &str,
    texture_folder: &str,
    mesh_name: &str,
) -> Result<()> {
    let file_name_binary = format!("{}.bin", mesh_name);
    let file_name_json = format!("{}.gltf", mesh_name);
    let dst_binary = File::create(format!("{}/{}", output_folder, file_name_binary))
        .context("could not create binary glTF data file")?;
    let dst_json = File::create(format!("{}/{}", output_folder, file_name_json))
        .context("could not create JSON glTF data file")?;

    export::mesh_to_binary(
        dst_binary,
        file_name_binary,
        dst_json,
        texture_folder,
        mesh,
        Some(mesh_name.to_string()),
    )
}

/// Note: texture_folder is relative to the output_folder.
/// E.g. output_folder = "output" and texture_folder = "textures" results in textures being in "output/textures".
fn create_rigged_gltf(
    meshes: &[(String, d3dmesh::Data)],
    output_folder: &str,
    texture_folder: &str,
    root_name: &str,
    skeleton: &Skeleton,
) -> Result<()> {
    let file_name_binary = format!("{}.bin", root_name);
    let file_name_json = format!("{}.gltf", root_name);
    let dst_binary = File::create(format!("{}/{}", output_folder, file_name_binary))
        .context("could not create binary glTF data file")?;
    let dst_json = File::create(format!("{}/{}", output_folder, file_name_json))
        .context("could not create JSON glTF data file")?;

    export::rigged_object_to_binary(
        dst_binary,
        file_name_binary,
        dst_json,
        texture_folder,
        Some(root_name.to_string()),
        meshes,
        skeleton,
    )
}

/// Copy necessary textures for the materials in the mesh.
/// Note: the texture_folder needs to be "absolute" (or relative to the executable).
/// i.e. including the output folder.
fn copy_textures(
    config: &Config,
    input_folder: &str,
    texture_folder_absolute: &Path,
    materials: &[d3dmesh::materials::Material],
) -> Result<()> {
    // copy necessary textures
    for material in materials {
        for texture in &material.textures {
            if texture.map == TextureMap::Map || texture.map == TextureMap::MapA {
                let texture_name = Path::new(&texture.name);
                let texture_path = texture_name.with_extension("png");
                let from = Path::new(input_folder).join(&texture_name);
                let to = Path::new(texture_folder_absolute).join(&texture_path);

                match texture.kind {
                    // simply copy textures without any conversion:
                    TextureType::Diffuse
                    | TextureType::Detail
                    | TextureType::Ink
                    | TextureType::Height => {
                        image_conversion::copy_texture(&from, &to)
                            .context(format!("could not copy texture: {}", &texture.name,))?;
                    }
                    // textures that need conversion:
                    TextureType::Normal => {
                        let new_normal = image_conversion::normal_map(&from).context(format!(
                            "could not convert normal map texture: {} (expected it in {:?})",
                            &texture.name, from,
                        ))?;
                        new_normal
                            .save(to)
                            .context("could not save new normal map")?;

                        // create displacement/height map from the normal map
                        if config.enable_height_map {
                            let height_path = texture_name
                                .file_stem()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .trim_end_matches("_nm");
                            let height_path = Path::new(texture_folder_absolute).join(
                                Path::new(&format!("{}_height", height_path)).with_extension("png"),
                            );
                            let height_map = image_conversion::height::normal_to_height(new_normal);
                            height_map
                                .save(height_path)
                                .context("could not save new height map")?;
                        }
                    }
                    TextureType::Specular => {
                        let new_specular =
                            image_conversion::specular_map(&from).context(format!(
                                "could not convert specular map texture: {} (expected it in {:?})",
                                &texture.name, from,
                            ))?;
                        new_specular
                            .save(to)
                            .context("could not save new specular map")?;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

/// Converts the given path to a string that contains the raw file-name without directories or extension.
fn get_file_name_from_path(path: &Path) -> Result<&str> {
    let file_name = path
        .file_stem()
        .ok_or(anyhow!("file does not have valid filename: {:?}", path))?
        .to_str()
        .ok_or(anyhow!(
            "file does not have a valid utf-8 filename: {:?}",
            path
        ))?;
    Ok(file_name)
}
