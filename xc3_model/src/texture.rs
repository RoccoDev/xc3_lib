use image_dds::{ddsfile::Dds, error::CreateImageError, CreateDdsError, Surface};
use log::error;
use thiserror::Error;
use xc3_lib::{
    mibl::{CreateMiblError, Mibl, SwizzleError},
    msrd::streaming::ExtractedTexture,
    mxmd::PackedTexture,
};

pub use xc3_lib::mibl::{ImageFormat, ViewDimension};
pub use xc3_lib::mxmd::TextureUsage;

#[derive(Debug)]
pub enum ExtractedTextures {
    Switch(Vec<ExtractedTexture<Mibl>>),
    Pc(Vec<ExtractedTexture<Dds>>),
}

#[derive(Debug, Error)]
pub enum CreateImageTextureError {
    #[error("error deswizzling surface: {0}")]
    Swizzle(#[from] SwizzleError),

    #[error("error reading data: {0}")]
    Binrw(#[from] binrw::Error),

    #[error("error decompressing stream: {0}")]
    Stream(#[from] xc3_lib::error::DecompressStreamError),

    #[error("error converting image surface: {0}")]
    Surface(#[from] image_dds::error::SurfaceError),

    #[error("error converting Mibl texture: {0}")]
    Mibl(#[from] xc3_lib::mibl::CreateMiblError),
}

/// A non swizzled version of an [Mibl] texture.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageTexture {
    /// An optional name assigned to some textures.
    /// This will typically be [None]
    /// and can not be used for reliably identifying textures.
    pub name: Option<String>,
    /// Hints on how the texture is used.
    /// Actual usage is determined by the shader code.
    pub usage: Option<TextureUsage>,
    /// The width of the base mip level in pixels.
    pub width: u32,
    /// The height of the base mip level in pixels.
    pub height: u32,
    /// The depth of the base mip level in pixels.
    pub depth: u32,
    pub view_dimension: ViewDimension, // TODO: is this redundant?
    pub image_format: ImageFormat,
    /// The number of mip levels or 1 if there are no mipmaps.
    pub mipmap_count: u32,
    /// The combined image surface data in a standard row-major layout.
    /// Ordered as `Layer 0 Mip 0, Layer 0 Mip 1, ... Layer L-1 Mip M-1`
    /// for L layers and M mipmaps similar to DDS files.
    pub image_data: Vec<u8>,
}

impl ImageTexture {
    /// Deswizzle the data from `mibl`.
    ///
    /// The `name` is not required but creates more descriptive file names and debug information.
    /// The `usage` improves the accuracy of texture assignments if the shader database is not specified.
    pub fn from_mibl(
        mibl: &Mibl,
        name: Option<String>,
        usage: Option<TextureUsage>,
    ) -> Result<Self, SwizzleError> {
        Ok(Self {
            name,
            usage,
            width: mibl.footer.width,
            height: mibl.footer.height,
            depth: mibl.footer.depth,
            view_dimension: mibl.footer.view_dimension,
            image_format: mibl.footer.image_format,
            mipmap_count: mibl.footer.mipmap_count,
            image_data: mibl.deswizzled_image_data()?,
        })
    }

    pub fn from_packed_texture(texture: &PackedTexture) -> Result<Self, CreateImageTextureError> {
        let mibl = Mibl::from_bytes(&texture.mibl_data)?;
        Self::from_mibl(&mibl, Some(texture.name.clone()), Some(texture.usage)).map_err(Into::into)
    }

    pub fn to_image(&self) -> Result<image_dds::image::RgbaImage, CreateImageError> {
        // Only decode the mip we actually use to improve performance.
        self.to_surface()
            .decode_layers_mipmaps_rgba8(0..self.layers(), 0..1)?
            .to_image(0)
    }

    /// Return the number of array layers in this surface.
    pub fn layers(&self) -> u32 {
        if self.view_dimension == ViewDimension::Cube {
            6
        } else {
            1
        }
    }

    pub fn to_surface(&self) -> image_dds::Surface<&[u8]> {
        Surface {
            width: self.width,
            height: self.height,
            depth: self.depth,
            layers: self.layers(),
            mipmaps: self.mipmap_count,
            image_format: self.image_format.into(),
            data: &self.image_data,
        }
    }

    // TODO: use a dedicated error type
    pub fn to_dds(&self) -> Result<Dds, CreateDdsError> {
        self.to_surface().to_dds()
    }

    pub fn from_surface<T: AsRef<[u8]>>(
        surface: Surface<T>,
        name: Option<String>,
        usage: Option<TextureUsage>,
    ) -> Result<Self, CreateImageTextureError> {
        Ok(Self {
            name,
            usage,
            width: surface.width,
            height: surface.height,
            depth: surface.depth,
            view_dimension: if surface.layers == 6 {
                ViewDimension::Cube
            } else if surface.depth > 1 {
                ViewDimension::D3
            } else {
                ViewDimension::D2
            },
            image_format: surface.image_format.try_into()?,
            mipmap_count: surface.mipmaps,
            image_data: surface.data.as_ref().to_vec(),
        })
    }

    pub fn from_dds(
        dds: &Dds,
        name: Option<String>,
        usage: Option<TextureUsage>,
    ) -> Result<Self, CreateImageTextureError> {
        Self::from_surface(Surface::from_dds(dds)?, name, usage)
    }

    pub fn to_mibl(&self) -> Result<Mibl, CreateMiblError> {
        Mibl::from_surface(self.to_surface())
    }
}

pub fn load_textures(
    textures: &ExtractedTextures,
) -> Result<Vec<ImageTexture>, CreateImageTextureError> {
    // TODO: what is the correct priority for the different texture sources?
    match textures {
        ExtractedTextures::Switch(textures) => textures
            .iter()
            .map(|texture| {
                ImageTexture::from_mibl(
                    &texture.mibl_final(),
                    Some(texture.name.clone()),
                    Some(texture.usage),
                )
                .map_err(Into::into)
            })
            .collect(),
        ExtractedTextures::Pc(textures) => textures
            .iter()
            .map(|texture| {
                ImageTexture::from_dds(
                    texture.dds_final(),
                    Some(texture.name.clone()),
                    Some(texture.usage),
                )
            })
            .collect(),
    }
}
