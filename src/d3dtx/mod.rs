mod bcn_image;

use std::io::{Read, Seek, SeekFrom};

use anyhow::{anyhow, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use image::{DynamicImage, GrayImage};

use crate::{
    byte_reading::{D3DName, VersionHeader},
    d3dtx::bcn_image::DxtDecoder,
};

use self::bcn_image::BCnVariant;

#[derive(Debug)]
pub struct Texture {
    pub name: String,
    pub image: DynamicImage,
}

impl Texture {
    pub fn parse<T: Read + Seek>(mut input: T) -> Result<Self> {
        let header = D3DTXHeader::parse(&mut input).context("could not parse D3DTX header")?;
        log::debug!("last mip-map start = {:#X}", &input.stream_position()?);

        let image: DynamicImage = match header.format {
            TextureFormat::BCn(variant) => {
                let decoder = DxtDecoder::new(input, header.width, header.height, variant)?;
                let image = decoder
                    .read_image()
                    .context("could not decode BCn image data")?;
                image
            }
            TextureFormat::A8 => {
                let mut data = Vec::new();
                input.read_to_end(&mut data)?;
                DynamicImage::ImageLuma8(
                    GrayImage::from_vec(header.width, header.height, data)
                        .ok_or(anyhow!("data buffer not big enough for A8 texture"))?,
                )
            }
            unknown => return Err(anyhow!("unknown TextureFormat: {:?}", unknown)),
        };

        Ok(Self {
            name: header.name,
            image,
        })
    }
}

/// The extracted header data from a .d3dtx file
struct D3DTXHeader {
    name: String,
    width: u32,
    height: u32,
    format: TextureFormat,
}

impl D3DTXHeader {
    pub fn parse<T: Read + Seek>(mut input: T) -> Result<Self> {
        let header = VersionHeader::parse(&mut input)?;
        match header {
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
        // skip unknowns
        input.seek(SeekFrom::Current(0x14))?;

        let name = D3DName::parse(&mut input)?;
        log::debug!("D3DTX file {}", name.clone().to_string());

        // skip unknowns
        input.seek(SeekFrom::Current(0x0C))?;

        let flag = input.read_u8()?;
        if flag == 0x31 {
            input.seek(SeekFrom::Current(0x08))?; // TODO: needs fixing?
            let header_jump = input.read_u32::<LittleEndian>()?;
            input.seek(SeekFrom::Current(header_jump as i64 - 4))?;
        }

        let mip_maps = input.read_u32::<LittleEndian>()?;
        let width = input.read_u32::<LittleEndian>()?;
        let height = input.read_u32::<LittleEndian>()?;
        // skip unknowns
        input.seek(SeekFrom::Current(0x08))?;
        let dxt_type = input.read_u32::<LittleEndian>()?;
        let format = TextureFormat::parse(dxt_type);

        // skip unknowns
        input.seek(SeekFrom::Current(0x5C))?;

        log::debug!(
            "mip map info start = {:#X}, mip_map_count = {}, width = {}, height = {}, format = {:?}",
            input.stream_position()?,
            mip_maps,
            width,
            height,
            format,
        );

        let mut mip_sizes = Vec::new();
        for _ in 0..mip_maps {
            // skip unknowns
            input.seek(SeekFrom::Current(0x0C))?;
            let mip_size = input.read_u32::<LittleEndian>()?;
            mip_sizes.push(mip_size);
            // skip unknowns
            input.seek(SeekFrom::Current(0x08))?;
        }

        log::debug!("data_start = {:#X}", input.stream_position()?);

        // skip mip-size data
        // the mip_sizes array contains the sizes in number of bytes for the mip header data
        // the mip_sizes array starts with the smallest mip, i.e. the biggest mip is at the end
        for mip_size in &mip_sizes[..mip_sizes.len() - 1] {
            input.seek(SeekFrom::Current(*mip_size as i64))?;
        }

        Ok(Self {
            name: name.to_string(),
            width,
            height,
            format,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum TextureFormat {
    BCn(BCnVariant),
    A8,
    Unknown(u32),
}

impl TextureFormat {
    fn parse(value: u32) -> Self {
        match value {
            16 | 17 => Self::A8,
            64 => Self::BCn(BCnVariant::BC1),
            65 => Self::BCn(BCnVariant::BC2), // TODO: only guess, check with real data!
            66 => Self::BCn(BCnVariant::BC3),
            67 => Self::BCn(BCnVariant::BC4),
            68 => Self::BCn(BCnVariant::BC5),
            _ => Self::Unknown(value),
        }
    }
}
