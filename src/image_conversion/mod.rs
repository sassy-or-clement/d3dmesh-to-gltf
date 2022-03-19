pub mod height;
mod sampler;

use std::{fs, io::Cursor, path::Path};

use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, Rgb, RgbImage, Rgba, RgbaImage};

use crate::d3dtx;

/// Checks whether or not the texture at the given path has useful alpha information.
/// If true, the texture has useful alpha information.
/// A texture with an alpha channel where all values are `0xFF` is treated as "no useful alpha values".
/// The result is false in this case.
pub fn texture_has_alpha_information<P: AsRef<Path>>(from: P) -> Result<bool> {
    let image = image::open(&from).context("could not open/decode texture for alpha check")?;

    // compute whether alpha values are present
    // Note that there is the result of `all` is often negated by a `!`
    let texture_has_alpha = match &image {
        DynamicImage::ImageBgra8(bgra) => !bgra
            .enumerate_pixels()
            .all(|(_, _, pixel)| pixel[3] == u8::MAX),
        DynamicImage::ImageRgba8(rgba) => !rgba
            .enumerate_pixels()
            .all(|(_, _, pixel)| pixel[3] == u8::MAX),
        DynamicImage::ImageRgba16(rgba) => !rgba
            .enumerate_pixels()
            .all(|(_, _, pixel)| pixel[3] == u16::MAX),
        DynamicImage::ImageLumaA8(la) => !la
            .enumerate_pixels()
            .all(|(_, _, pixel)| pixel[1] == u8::MAX),
        DynamicImage::ImageLumaA16(la) => !la
            .enumerate_pixels()
            .all(|(_, _, pixel)| pixel[1] == u16::MAX),
        _ => false,
    };

    Ok(texture_has_alpha)
}

/// Reads a texture and writes it without any modifications to its content to the destination.
/// Might perform format conversion, though.
pub fn copy_texture<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<()> {
    let image = open_d3dtx(&from).context("could not open/decode texture")?;

    image.save(to).context("could not save texture")?;
    Ok(())
}

/// Reads in a normal map from The Walking Dead: The Telltale Definitive Series and
/// converts it to a typical three-component normal map with R: X; G: Y; B: Z.
pub fn normal_map<P: AsRef<Path>>(from: P) -> Result<RgbImage> {
    let image = open_d3dtx(&from).context("could not decode normal map image")?;
    let new_normal = match image {
        DynamicImage::ImageRgba8(rgba) => {
            // some normal maps are RGBA and the channels are swopped
            let mut new_normal = RgbImage::new(rgba.width(), rgba.height());
            rgba.enumerate_pixels().for_each(|(x, y, pixel)| {
                // Switch the channels as following:
                // Blue -> Green
                // Green -> Blue
                // Alpha -> Red
                // Note: invert the red channel, which makes it compatible to the glTF normal map format
                let new_pixel: Rgb<u8> = [255 - pixel[3], pixel[2], pixel[1]].into();
                new_normal.put_pixel(x, y, new_pixel);
            });
            new_normal
        }
        DynamicImage::ImageLumaA8(rg) => {
            // normals are compressed via two channels that only contain x and y
            // https://developer.download.nvidia.com/whitepapers/2008/real-time-normal-map-dxt-compression.pdf
            // chapter 3.3 Tangent-Space 3Dc
            let mut new_normal = RgbImage::new(rg.width(), rg.height());
            rg.enumerate_pixels().for_each(|(x, y, pixel)| {
                // Note: one u8 holds a value in the range [-1; 1] by storing it in the byte range [0; 255]
                // i.e. 127 is actually the value 0 and 255 is 1 and 0 is -1
                let normal_x = u8_to_f32_norm(pixel[0]);
                let normal_y = u8_to_f32_norm(pixel[1]);
                // input from texture is [0; 1] and mapped to [-1; 1]
                let normal_x = (normal_x * 2.0) - 1.0;
                let normal_y = (normal_y * 2.0) - 1.0;
                // calculate z by using sqrt(1-x²-y²)
                let normal_z = f32::sqrt(1.0 - (normal_x * normal_x) - (normal_y * normal_y));
                // map x, y and z from the previous range [-1; 1] to [0; 1]
                let normal_x = (normal_x + 1.0) / 2.0;
                let normal_y = (normal_y + 1.0) / 2.0;
                let normal_z = (normal_z + 1.0) / 2.0;
                let new_pixel: Rgb<u8> = [
                    f32_to_u8_norm(normal_x),
                    f32_to_u8_norm(normal_y),
                    f32_to_u8_norm(normal_z),
                ]
                .into();
                new_normal.put_pixel(x, y, new_pixel);
            });
            new_normal
        }
        // Rgb8 format is not converted under the assumption that the format is already correct
        DynamicImage::ImageRgb8(rgb) => rgb,
        val => return Err(anyhow!("unknown normal map format {:?}", val.color())),
    };

    Ok(new_normal)
}

/// Reads in a spec-map from The Walking Dead: The Telltale Definitive Series and
/// converts it to a glTF texture with the following setup:
///
/// 1. R: Occlusion
/// 2. G: Roughness
/// 3. B: Metalness
/// 4. A: Specular
pub fn specular_map<P: AsRef<Path>>(from: P) -> Result<RgbaImage> {
    let image = open_d3dtx(&from).context("could not decode specular map image")?;

    let image = match image {
        image::DynamicImage::ImageRgba8(rgba) => rgba,
        image::DynamicImage::ImageRgb8(rgb) => {
            // convert from RGB to RGBA with A as full roughness (=1)
            let mut rgba = RgbaImage::new(rgb.width(), rgb.height());
            rgb.enumerate_pixels().for_each(|(x, y, pixel)| {
                let new_pixel: Rgba<u8> = [pixel[0], pixel[1], pixel[2], 0].into();
                rgba.put_pixel(x, y, new_pixel)
            });
            rgba
        }
        val => return Err(anyhow!("unknown specular map format {:?}", val.color())),
    };

    let mut new_specular = RgbaImage::new(image.width(), image.height());
    image.enumerate_pixels().for_each(|(x, y, pixel)| {
        // The Telltale setup is internally:
        // R: Specular (?)
        // G: Metalness
        // B: 1-Occlusion
        // A: 1-Roughness (sometimes called glossy)
        let new_pixel: Rgba<u8> = [255 - pixel[2], 255 - pixel[3], pixel[1], pixel[0]].into();
        new_specular.put_pixel(x, y, new_pixel);
    });

    Ok(new_specular)
}

fn open_d3dtx<P: AsRef<Path>>(path: P) -> Result<DynamicImage> {
    let file = fs::read(&path).context(format!(
        "could not open d3dtx file (expected at {})",
        path.as_ref().to_string_lossy()
    ))?;
    let input = Cursor::new(file);
    let image = d3dtx::Texture::parse(input)?;
    Ok(image.image)
}

// converts a u8 in the range [0, 255] to a f32 in the range [0, 1].
fn u8_to_f32_norm(value: u8) -> f32 {
    value as f32 / 255.0
}

// converts a f32 in the range [0, 1] to a u8 in the range [0, 255].
fn f32_to_u8_norm(value: f32) -> u8 {
    (value * 255.0) as u8
}
