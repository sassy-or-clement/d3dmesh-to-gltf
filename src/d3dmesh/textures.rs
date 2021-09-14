use std::io::Read;

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};

use crate::checksum_mapping::ChecksumMap;

#[derive(Debug)]
pub struct Texture {
    pub kind: TextureType,
    pub map: TextureMap,
    pub name: String,
}

impl Texture {
    pub fn parse<T: Read>(mut input: T, texture_mapping: &ChecksumMap) -> Result<Self> {
        let type_hash = input.read_u64::<LittleEndian>()?;

        let (texture_type, texture_map) = match type_hash {
            0x9836970882a34f02 => (TextureType::Anisotropy, TextureMap::Map),
            0x714d23445936b35d => (TextureType::AnisotropyMask, TextureMap::Map),
            0x7501e041ac72a988 => (TextureType::AnisotropyTangent, TextureMap::Map),
            0xb8b04ddf1796f446 => (TextureType::Bump, TextureMap::Map),
            0x72507eea6ef21aee => (TextureType::ColorMask, TextureMap::Map),
            0x2b6c47845f607734 => (TextureType::DamageMask, TextureMap::MapA),
            0xec7d65b8a55e2c81 => (TextureType::DamageMask, TextureMap::MapB),
            0x36170f97445b6e2e => (TextureType::DecalDiffuse, TextureMap::Map),
            0xa1f1257a331854c4 => (TextureType::DecalMask, TextureMap::Map),
            0x9cf676c6403c9784 => (TextureType::DecalNormal, TextureMap::Map),
            0x4930b970a7fd511f => (TextureType::Detail, TextureMap::Map),
            0xdf7e412256e87e74 => (TextureType::Detail, TextureMap::MapB),
            0xcb433436edca9efb => (TextureType::DetailGloss, TextureMap::Map),
            0xbf468ef480aeeb89 => (TextureType::DetailMask, TextureMap::Map),
            0x963ee638083014f1 => (TextureType::DetailNormal, TextureMap::Map),
            0x706cf2aa57a7a206 => (TextureType::DetailNormal, TextureMap::Map),
            0xd49d30f64a580c6f => (TextureType::DetailNormal, TextureMap::MapA),
            0x138c12cab06657da => (TextureType::DetailNormal, TextureMap::MapB),
            0x517cf321198c6149 => (TextureType::DetailNormal, TextureMap::MapC),
            0xbdcd25f20f4199e3 => (TextureType::PackedDetail, TextureMap::Map),
            0x8648fa82d1dbee1a => (TextureType::Diffuse, TextureMap::Map),
            0x94a590de74b1f5c1 => (TextureType::Diffuse, TextureMap::MapB),
            0xdc6e83a0253f163a => (TextureType::DiffuseLOD, TextureMap::Map),
            0xb3022ea7fd418b40 => (TextureType::Emission, TextureMap::Map),
            0xbdb4c92a546fb889 => (TextureType::Emission, TextureMap::MapB),
            0x13eee65865dfc90f => (TextureType::Environment, TextureMap::Map),
            0x257c2a45683f7d2f => (TextureType::Environment, TextureMap::Map),
            0x8cadb26098df1108 => (TextureType::Flow, TextureMap::Map),
            0x64fba83e34dd3959 => (TextureType::Gloss, TextureMap::Map),
            0x2642d6b4c8eccaa9 => (TextureType::Gradient, TextureMap::Map),
            0xa334f76c317a0c02 => (TextureType::Gradient, TextureMap::Map),
            0x2aa89260d8661f89 => (TextureType::Grime, TextureMap::Map),
            0x66cd6e57fa58a246 => (TextureType::Height, TextureMap::Map),
            0xff787a61eac8a5b5 => (TextureType::Ink, TextureMap::Map),
            0x817afd5302445b8b => (TextureType::MicrodetailDiffuse, TextureMap::Map),
            0xcb5b9a7f52168a41 => (TextureType::MicrodetailNormal, TextureMap::Map),
            0x1e3f6b9f2550389d => (TextureType::Normal, TextureMap::Map),
            0x3f380050afd9f81f => (TextureType::Normal, TextureMap::MapB),
            0x436206e68a9e7cca => (TextureType::Normal, TextureMap::MapB),
            0x7498a5f1b80ad419 => (TextureType::NormalAlternate, TextureMap::Map),
            0xcaaae6432af348c0 => (TextureType::Occlusion, TextureMap::Map),
            0x62c4957578189f07 => (TextureType::Occlusion, TextureMap::Map),
            0x533f479d08bf0e5e => (TextureType::RainFall, TextureMap::Map),
            0x2eba1f4bba7a1543 => (TextureType::RainWet, TextureMap::Map),
            0xc8c94155fb7c634b => (TextureType::Specular, TextureMap::Map),
            0xd5b57775db361670 => (TextureType::Specular, TextureMap::Map),
            0x120621d5fad4c090 => (TextureType::Specular, TextureMap::MapB),
            0x37571b60b1f61180 => (TextureType::Tangent, TextureMap::MapB),
            0xa45200a222dc2d80 => (TextureType::Thickness, TextureMap::Map),
            0x8cf38a5266aaa7a4 => (TextureType::TransitionNormal, TextureMap::Map),
            0x87b579ec018fbd4d => (TextureType::VisibilityMask, TextureMap::Map),
            0xd7ea35534dbc457d => (TextureType::WrinkleMask, TextureMap::MapA),
            0x10fb176fb7821ec8 => (TextureType::WrinkleMask, TextureMap::MapB),
            0xf340c5690ce9e059 => (TextureType::WrinkleNormal, TextureMap::Map),
            0xa13d14fbb436f23b => (TextureType::WrinkleNormal, TextureMap::Map),
            _ => (TextureType::Unknown, TextureMap::Unknown),
        };

        let texture_hash = input.read_u64::<LittleEndian>()?;
        let texture_name = texture_mapping.get_mapping(texture_hash);
        let texture_name = if let Some(name) = texture_name {
            name
        } else {
            // Note: sometimes the checksum is actually 0?! just ignore then
            if texture_hash != 0 {
                log::warn!(
                    "Warning: could not resolve texture ID to name: {:016x}",
                    texture_hash
                );
            }
            return Ok(Self {
                kind: TextureType::Unknown,
                map: TextureMap::Unknown,
                name: "".to_string(),
            });
        };

        log::debug!(
            "Texture: {:016x}: {:?} - {:?} {:?}",
            texture_hash,
            texture_name,
            texture_type,
            texture_map,
        );

        Ok(Self {
            kind: texture_type,
            map: texture_map,
            name: texture_name,
        })
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureType {
    Anisotropy,
    AnisotropyMask,
    AnisotropyTangent,
    Bump,
    ColorMask,
    DamageMask,
    DecalDiffuse,
    DecalMask,
    DecalNormal,
    Detail,
    DetailGloss,
    DetailMask,
    DetailNormal,
    PackedDetail,
    Diffuse,
    DiffuseLOD,
    Emission,
    Environment,
    Flow,
    Gloss,
    Gradient,
    Grime,
    Height,
    Ink,
    MicrodetailDiffuse,
    MicrodetailNormal,
    Normal,
    NormalAlternate,
    Occlusion,
    RainFall,
    RainWet,
    Specular,
    Tangent,
    Thickness,
    TransitionNormal,
    VisibilityMask,
    WrinkleMask,
    WrinkleNormal,
    Unknown,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureMap {
    Map,
    MapA,
    MapB,
    MapC,
    Unknown,
}
