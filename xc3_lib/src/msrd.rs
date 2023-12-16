//! Streamed model resources like shaders, geometry, or textures in `.wismt` files.
//!
//! # File Paths
//! | Game | File Patterns |
//! | --- | --- |
//! | Xenoblade Chronicles 1 DE | `chr/{en,np,obj,pc,wp}/*.wismt` |
//! | Xenoblade Chronicles 2 | `model/{bl,en,np,oj,pc,we,wp}/*.wismt` |
//! | Xenoblade Chronicles 3 | `chr/{bt,ch,en,oj,wp}/*.wismt`, `map/*.wismt` |
use std::{
    borrow::Cow,
    io::{Cursor, Seek, Write},
};

use crate::{
    dds::DdsExt,
    error::DecompressStreamError,
    mibl::Mibl,
    mxmd::{PackedExternalTexture, PackedExternalTextures, TextureUsage},
    parse_count32_offset32, parse_opt_ptr32, parse_ptr32,
    spch::Spch,
    vertex::VertexData,
    xbc1::Xbc1,
    xc3_write_binwrite_impl,
};
use bilge::prelude::*;
use binrw::{args, binread, BinRead, BinWrite};
use image_dds::ddsfile::Dds;
use xc3_write::{round_up, write_full, Xc3Write, Xc3WriteOffsets};

// TODO: find a way to share the stream type with mxmd
// TODO: how to set the offsets when repacking the msrd?
#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"DRSM"))]
#[xc3(magic(b"DRSM"))]
pub struct Msrd {
    /// Version `10001`
    pub version: u32,

    // TODO: Can this be calculated without writing the data?
    // rounded or aligned in some way?
    pub header_size: u32, // TODO: xbc1 offset - 16?

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub data: Streaming<Stream>,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Streaming<S>
where
    S: Xc3Write + 'static,
    for<'a> <S as Xc3Write>::Offsets<'a>: Xc3WriteOffsets,
    for<'a> S: BinRead<Args<'a> = ()>,
{
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(args_raw(base_offset))]
    pub inner: StreamingInner<S>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub enum StreamingInner<S>
where
    S: Xc3Write + 'static,
    for<'b> <S as Xc3Write>::Offsets<'b>: Xc3WriteOffsets,
    for<'a> S: BinRead<Args<'a> = ()>,
{
    #[br(magic(0u32))]
    #[xc3(magic(0u32))]
    StreamingLegacy(#[br(args_raw(base_offset))] StreamingDataLegacy),

    #[br(magic(4097u32))]
    #[xc3(magic(4097u32))]
    Streaming(#[br(args_raw(base_offset))] StreamingData<S>),
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(import_raw(base_offset: u64))]
pub struct StreamingDataLegacy {
    pub flags: StreamingFlagsLegacy,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub low_textures: PackedExternalTextures,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub textures: Option<PackedExternalTextures>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: low_textures.textures.len() }})]
    #[xc3(offset(u32))]
    pub low_texture_indices: Vec<u16>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! { count: textures.as_ref().map(|t| t.textures.len()).unwrap_or_default() }
    })]
    #[xc3(offset(u32))]
    pub texture_indices: Option<Vec<u16>>,

    pub low_texture_data_offset: u32,
    pub texture_data_offset: u32,

    pub low_texture_data_compressed_size: u32,
    pub texture_data_compressed_size: u32,

    pub low_texture_data_uncompressed_size: u32,
    pub texture_data_uncompressed_size: u32,
}

/// Flags indicating the way data is stored in the model's `wismt` file.
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u32))]
pub enum StreamingFlagsLegacy {
    Uncompressed = 1,
    Xbc1 = 2,
}

// 76 (xc1, xc2, xc3) or 92 (xc3) bytes.
#[binread]
#[derive(Debug, Xc3Write)]
#[br(import_raw(base_offset: u64))]
pub struct StreamingData<S>
where
    S: Xc3Write + 'static,
    for<'a> S: BinRead<Args<'a> = ()>,
{
    pub stream_flags: StreamFlags,

    // Used for estimating the struct size.
    #[br(temp, restore_position)]
    offset: (u32, u32),

    /// Files contained within [streams](#structfield.streams).
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub stream_entries: Vec<StreamEntry>,

    // TODO: Document the typical ordering of streams?
    /// A collection of [Xbc1] streams with decompressed layout
    /// specified in [stream_entries](#structfield.stream_entries).
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub streams: Vec<S>,

    /// The [StreamEntry] for [Msrd::extract_vertex_data] with [EntryType::Vertex].
    pub vertex_data_entry_index: u32,
    /// The [StreamEntry] for [Msrd::extract_shader_data] with [EntryType::Shader].
    pub shader_entry_index: u32,

    /// The [StreamEntry] for [Msrd::extract_low_textures] with [EntryType::LowTextures].
    pub low_textures_entry_index: u32,
    /// The [Stream] for [Msrd::extract_low_textures].
    pub low_textures_stream_index: u32,

    /// The [Stream] for [Msrd::extract_textures].
    pub textures_stream_index: u32,
    /// The first [StreamEntry] for [Msrd::extract_textures].
    pub textures_stream_entry_start_index: u32,
    /// The number of [StreamEntry] corresponding
    /// to the number of textures in [Msrd::extract_textures].
    pub textures_stream_entry_count: u32,

    #[br(args { base_offset, size: offset.1 })]
    pub texture_resources: TextureResources,
}

// TODO: Better name?
// TODO: Always identical to mxmf?
#[derive(Debug, BinRead, Xc3Write, PartialEq)]
#[br(import { base_offset: u64, size: u32 })]
pub struct TextureResources {
    // TODO: also used for chr textures?
    /// Index into [low_textures](#structfield.low_textures)
    /// for each of the textures in [Msrd::extract_textures](crate::msrd::Msrd::extract_textures).
    /// This allows assigning higher resolution versions to only some of the textures.
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub texture_indices: Vec<u16>,

    // TODO: Some of these use actual names?
    // TODO: Possible to figure out the hash function used?
    /// Name and data range for each of the [Mibl] textures.
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32), align(2))]
    pub low_textures: Option<PackedExternalTextures>,

    /// Always `0`.
    pub unk1: u32,

    // TODO: only used for some xc3 models with chr/tex textures?
    #[br(if(size == 92), args_raw(base_offset))]
    pub chr_textures: Option<ChrTexTextures>,

    // TODO: padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq)]
#[br(import_raw(base_offset: u64))]
pub struct ChrTexTextures {
    #[br(parse_with = parse_count32_offset32, offset = base_offset)]
    #[xc3(count_offset(u32, u32))]
    pub chr_textures: Vec<ChrTexTexture>,

    // TODO: additional padding?
    pub unk: [u32; 2],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq)]
pub struct ChrTexTexture {
    // TODO: The texture name hash as an integer for xc3?
    pub hash: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

/// A file contained in a [Stream].
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone)]
pub struct StreamEntry {
    /// The offset in bytes for the decompressed data range in the stream.
    pub offset: u32,
    /// The size in bytes of the decompressed data range in the stream.
    pub size: u32,
    /// Index into [streams](struct.StreamingData.html#structfield.streams)
    /// for the high resolution base mip level starting from 1.
    /// Has no effect if [entry_type](#structfield.entry_type) is not [EntryType::Texture]
    /// or the index is 0.
    pub texture_base_mip_stream_index: u16,
    pub entry_type: EntryType,
    // TODO: padding?
    pub unk: [u32; 2],
}

/// Flags indicating what stream data is present.
#[bitsize(32)]
#[derive(DebugBits, FromBits, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct StreamFlags {
    pub has_vertex: bool,
    pub has_spch: bool,
    pub has_low_textures: bool,
    pub has_textures: bool,
    pub unk5: bool,
    pub unk6: bool,
    pub has_chr_textures: bool,
    pub unk: u25,
}

/// The type of data for a [StreamEntry].
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u16))]
pub enum EntryType {
    /// A single [VertexData].
    Vertex = 0,
    /// A single [Spch].
    Shader = 1,
    /// A collection of [Mibl].
    LowTextures = 2,
    /// A single [Mibl].
    Texture = 3,
}

/// A compressed [Xbc1] stream with items determined by [StreamEntry].
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Stream {
    /// The size of [xbc1](#structfield.xbc1), including the header.
    pub compressed_size: u32,
    /// The size of the decompressed data in [xbc1](#structfield.xbc1).
    /// Aligned to 4096 (0x1000).
    pub decompressed_size: u32,
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub xbc1: Xbc1,
}

impl Stream {
    pub fn from_xbc1(xbc1: Xbc1) -> Self {
        // TODO: Should this make sure the xbc1 decompressed data is actually aligned?
        Self {
            compressed_size: (round_up(xbc1.compressed_stream.len() as u64, 16) + 48) as u32,
            decompressed_size: round_up(xbc1.decompressed_size as u64, 4096) as u32,
            xbc1,
        }
    }
}

// TODO: move to a stream.rs submodule?
// TODO: Add a function to create an extractedtexture from a surface?
#[derive(Debug)]
pub struct ExtractedTexture<T> {
    pub name: String,
    pub usage: TextureUsage,
    pub low: T,
    pub high: Option<HighTexture<T>>,
}

#[derive(Debug, Clone)]
pub struct HighTexture<T> {
    pub mid: T,
    pub base_mip: Option<Vec<u8>>,
}

impl ExtractedTexture<Dds> {
    /// Returns the highest possible quality [Dds] after trying low, high, or high + base mip level.
    pub fn dds_final(&self) -> &Dds {
        // TODO: Try and get the base mip level to work?
        // TODO: use a surface instead?
        self.high.as_ref().map(|h| &h.mid).unwrap_or(&self.low)
    }
}

impl ExtractedTexture<Mibl> {
    /// Returns the highest possible quality [Mibl] after trying low, high, or high + base mip level.
    /// Only high + base mip level returns [Cow::Owned].
    pub fn mibl_final(&self) -> Cow<'_, Mibl> {
        self.high
            .as_ref()
            .map(|h| {
                h.base_mip
                    .as_ref()
                    .map(|base| Cow::Owned(h.mid.with_base_mip(base)))
                    .unwrap_or(Cow::Borrowed(&h.mid))
            })
            .unwrap_or(Cow::Borrowed(&self.low))
    }
}

impl Msrd {
    pub fn decompress_stream(
        &self,
        stream_index: u32,
        entry_index: u32,
    ) -> Result<Vec<u8>, DecompressStreamError> {
        match &self.data.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.decompress_stream(stream_index, entry_index),
        }
    }

    // TODO: also add these methods to StreamingData<Stream>?
    /// Extract geometry for `wismt` and `pcsmt` files.
    pub fn extract_vertex_data(&self) -> Result<VertexData, DecompressStreamError> {
        match &self.data.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.extract_vertex_data(),
        }
    }

    /// Extract all textures for `wismt`` files.
    pub fn extract_textures(&self) -> Result<Vec<ExtractedTexture<Mibl>>, DecompressStreamError> {
        match &self.data.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.extract_textures(),
        }
    }

    // TODO: share code with above?
    /// Extract high resolution textures for `pcsmt` files.
    pub fn extract_pc_textures(&self) -> Result<Vec<ExtractedTexture<Dds>>, DecompressStreamError> {
        match &self.data.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.extract_pc_textures(),
        }
    }

    /// Extract shader programs for `wismt` and `pcsmt` files.
    pub fn extract_shader_data(&self) -> Result<Spch, DecompressStreamError> {
        match &self.data.inner {
            StreamingInner::StreamingLegacy(_) => todo!(),
            StreamingInner::Streaming(data) => data.extract_shader_data(),
        }
    }
}

impl StreamingData<Stream> {
    pub fn decompress_stream(
        &self,
        stream_index: u32,
        entry_index: u32,
    ) -> Result<Vec<u8>, DecompressStreamError> {
        let stream = &self.streams[stream_index as usize].xbc1.decompress()?;
        let entry = &self.stream_entries[entry_index as usize];
        Ok(stream[entry.offset as usize..entry.offset as usize + entry.size as usize].to_vec())
    }

    /// Extract geometry for `wismt` and `pcsmt` files.
    pub fn extract_vertex_data(&self) -> Result<VertexData, DecompressStreamError> {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.vertex_data_entry_index)?;
        VertexData::from_bytes(bytes).map_err(Into::into)
    }

    fn extract_low_textures(&self) -> Result<Vec<ExtractedTexture<Mibl>>, DecompressStreamError> {
        let bytes = self.decompress_stream(
            self.low_textures_stream_index,
            self.low_textures_entry_index,
        )?;

        match &self.texture_resources.low_textures {
            Some(low_textures) => low_textures
                .textures
                .iter()
                .map(|t| {
                    let mibl_bytes = &bytes
                        [t.mibl_offset as usize..t.mibl_offset as usize + t.mibl_length as usize];
                    Ok(ExtractedTexture {
                        name: t.name.clone(),
                        usage: t.usage,
                        low: Mibl::from_bytes(mibl_bytes)?,
                        high: None,
                    })
                })
                .collect(),
            None => Ok(Vec::new()),
        }
    }

    fn extract_low_pc_textures(&self) -> Vec<ExtractedTexture<Dds>> {
        // TODO: Avoid unwrap.
        let bytes = self
            .decompress_stream(
                self.low_textures_stream_index,
                self.low_textures_entry_index,
            )
            .unwrap();

        match &self.texture_resources.low_textures {
            Some(low_textures) => low_textures
                .textures
                .iter()
                .map(|t| {
                    let dds_bytes = &bytes
                        [t.mibl_offset as usize..t.mibl_offset as usize + t.mibl_length as usize];

                    ExtractedTexture {
                        name: t.name.clone(),
                        usage: t.usage,
                        low: Dds::read(dds_bytes).unwrap(),
                        high: None,
                    }
                })
                .collect(),
            None => Vec::new(),
        }
    }

    // TODO: avoid unwrap?
    /// Extract all textures for `wismt` files.
    pub fn extract_textures(&self) -> Result<Vec<ExtractedTexture<Mibl>>, DecompressStreamError> {
        self.extract_textures_inner(
            |s| s.extract_low_textures().unwrap(),
            |b| Mibl::from_bytes(b).unwrap(),
        )
    }

    /// Extract high resolution textures for `pcsmt` files.
    pub fn extract_pc_textures(&self) -> Result<Vec<ExtractedTexture<Dds>>, DecompressStreamError> {
        self.extract_textures_inner(Self::extract_low_pc_textures, |b| {
            Dds::from_bytes(b).unwrap()
        })
    }

    fn extract_textures_inner<T, F1, F2>(
        &self,
        read_low: F1,
        read_t: F2,
    ) -> Result<Vec<ExtractedTexture<T>>, DecompressStreamError>
    where
        F1: Fn(&Self) -> Vec<ExtractedTexture<T>>,
        F2: Fn(&[u8]) -> T,
    {
        // Start with no high res textures or base mip levels.
        let mut textures = read_low(self);

        // The high resolution textures are packed into a single stream.
        let stream = &self.streams[self.textures_stream_index as usize]
            .xbc1
            .decompress()?;

        let start = self.textures_stream_entry_start_index as usize;
        let count = self.textures_stream_entry_count as usize;
        for (i, entry) in self
            .texture_resources
            .texture_indices
            .iter()
            .zip(self.stream_entries[start..start + count].iter())
        {
            let bytes = &stream[entry.offset as usize..entry.offset as usize + entry.size as usize];
            let mid = read_t(bytes);

            // Indices start from 1 for the base mip level.
            let base_mip_stream_index = entry.texture_base_mip_stream_index.saturating_sub(1);
            let base_mip = if base_mip_stream_index != 0 {
                Some(
                    self.streams[base_mip_stream_index as usize]
                        .xbc1
                        .decompress()?,
                )
            } else {
                None
            };

            textures[*i as usize].high = Some(HighTexture { mid, base_mip });
        }

        Ok(textures)
    }

    /// Extract shader programs for `wismt` and `pcsmt` files.
    pub fn extract_shader_data(&self) -> Result<Spch, DecompressStreamError> {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.shader_entry_index)?;
        Spch::from_bytes(bytes).map_err(Into::into)
    }

    // TODO: This needs to create the entire Msrd since each stream offset depends on the header size?
    /// Pack and compress the files into new archive data.
    pub fn from_unpacked_files(
        vertex: &VertexData,
        spch: &Spch,
        textures: &[ExtractedTexture<Mibl>],
    ) -> Self {
        // TODO: handle other streams.
        let (stream_entries, streams, low_textures) = create_streams(vertex, spch, textures);

        // TODO: Search stream entries to get indices?
        // TODO: How are entry indices set if there are no textures?
        StreamingData {
            stream_flags: StreamFlags::new(
                true,
                true,
                true,
                false,
                false,
                false,
                false,
                0u8.into(),
            ),
            stream_entries,
            streams,
            vertex_data_entry_index: 0,
            shader_entry_index: 1,
            low_textures_entry_index: 2,
            low_textures_stream_index: 0, // TODO: always 0?
            textures_stream_index: 0,     // TODO: always 1 if textures are present?
            textures_stream_entry_start_index: 0,
            textures_stream_entry_count: 0,
            // TODO: How to properly create these fields?
            texture_resources: TextureResources {
                texture_indices: textures
                    .iter()
                    .enumerate()
                    .filter_map(|(i, t)| t.high.as_ref().map(|_| i as u16))
                    .collect(),
                low_textures: (!low_textures.is_empty()).then_some(PackedExternalTextures {
                    textures: low_textures,
                    unk2: 0,
                    strings_offset: 0,
                }),
                unk1: 0,
                chr_textures: None,
                unk: [0; 2],
            },
        }
    }
}

fn create_streams(
    vertex: &VertexData,
    spch: &Spch,
    textures: &[ExtractedTexture<Mibl>],
) -> (Vec<StreamEntry>, Vec<Stream>, Vec<PackedExternalTexture>) {
    // Entries are in ascending order by offset and stream.
    // Data order is Vertex, Shader, LowTextures, Textures.
    let mut streams = Vec::new();
    let mut stream_entries = Vec::new();

    let low_textures = write_stream0(&mut streams, &mut stream_entries, vertex, spch, textures);

    let entry_start_index = stream_entries.len();
    write_stream1(&mut streams, &mut stream_entries, textures);

    write_base_mip_streams(
        &mut streams,
        &mut stream_entries,
        textures,
        entry_start_index,
    );

    (stream_entries, streams, low_textures)
}

fn write_stream0(
    streams: &mut Vec<Stream>,
    stream_entries: &mut Vec<StreamEntry>,
    vertex: &VertexData,
    spch: &Spch,
    textures: &[ExtractedTexture<Mibl>],
) -> Vec<PackedExternalTexture> {
    // Data in streams is tightly packed.
    let mut writer = Cursor::new(Vec::new());
    stream_entries.push(write_stream_data(&mut writer, vertex, EntryType::Vertex));
    stream_entries.push(write_stream_data(&mut writer, spch, EntryType::Shader));

    let (entry, low_textures) = write_low_textures(&mut writer, textures);
    stream_entries.push(entry);

    let xbc1 = Xbc1::from_decompressed("0000".to_string(), &writer.into_inner()).unwrap();
    let stream = Stream::from_xbc1(xbc1);

    streams.push(stream);

    low_textures
}

fn write_stream1(
    streams: &mut Vec<Stream>,
    stream_entries: &mut Vec<StreamEntry>,
    textures: &[ExtractedTexture<Mibl>],
) {
    // Add higher resolution textures.
    let mut writer = Cursor::new(Vec::new());

    for texture in textures {
        if let Some(high) = &texture.high {
            let entry = write_stream_data(&mut writer, &high.mid, EntryType::Texture);
            stream_entries.push(entry);
        }
    }

    let xbc1 = Xbc1::from_decompressed("0000".to_string(), &writer.into_inner()).unwrap();
    let stream = Stream::from_xbc1(xbc1);
    streams.push(stream);
}

fn write_base_mip_streams(
    streams: &mut Vec<Stream>,
    stream_entries: &mut [StreamEntry],
    textures: &[ExtractedTexture<Mibl>],
    entry_start_index: usize,
) {
    // Only count textures with a higher resolution version to match entry ordering.
    for (i, high) in textures.iter().filter_map(|t| t.high.as_ref()).enumerate() {
        if let Some(base) = &high.base_mip {
            stream_entries[entry_start_index + i].texture_base_mip_stream_index =
                streams.len() as u16 + 1;

            // TODO: Should this be aligned in any way?
            let xbc1 = Xbc1::from_decompressed("0000".to_string(), base).unwrap();
            streams.push(Stream::from_xbc1(xbc1));
        }
    }
}

fn write_stream_data<'a, T>(
    writer: &mut Cursor<Vec<u8>>,
    data: &'a T,
    item_type: EntryType,
) -> StreamEntry
where
    T: Xc3Write + 'static,
    T::Offsets<'a>: Xc3WriteOffsets,
{
    let offset = writer.stream_position().unwrap();
    write_full(data, writer, 0, &mut 0).unwrap();
    let end_offset = writer.stream_position().unwrap();

    // Stream data is aligned to 4096 bytes.
    // TODO: Create a function for padding to an alignment?
    let size = end_offset - offset;
    let desired_size = round_up(size, 4096);
    let padding = desired_size - size;
    writer.write_all(&vec![0u8; padding as usize]).unwrap();
    let end_offset = writer.stream_position().unwrap();

    StreamEntry {
        offset: offset as u32,
        size: (end_offset - offset) as u32,
        texture_base_mip_stream_index: 0,
        entry_type: item_type,
        unk: [0; 2],
    }
}

fn write_low_textures(
    writer: &mut Cursor<Vec<u8>>,
    textures: &[ExtractedTexture<Mibl>],
) -> (StreamEntry, Vec<PackedExternalTexture>) {
    let mut low_textures = Vec::new();

    let offset = writer.stream_position().unwrap();
    for texture in textures {
        let mibl_offset = writer.stream_position().unwrap();
        texture.low.write(writer).unwrap();
        let mibl_length = writer.stream_position().unwrap() - mibl_offset;

        low_textures.push(PackedExternalTexture {
            usage: texture.usage,
            mibl_length: mibl_length as u32,
            mibl_offset: mibl_offset as u32 - offset as u32,
            name: texture.name.clone(),
        })
    }
    let end_offset = writer.stream_position().unwrap();

    // Assume the Mibl already have the required 4096 byte alignment.
    (
        StreamEntry {
            offset: offset as u32,
            size: (end_offset - offset) as u32,
            texture_base_mip_stream_index: 0,
            entry_type: EntryType::LowTextures,
            unk: [0; 2],
        },
        low_textures,
    )
}

xc3_write_binwrite_impl!(StreamEntry, StreamFlags, StreamingFlagsLegacy);

impl<'a, S> Xc3WriteOffsets for StreamingDataOffsets<'a, S>
where
    S: Xc3Write + 'static,
    for<'b> <S as Xc3Write>::Offsets<'b>: Xc3WriteOffsets,
    for<'b> S: BinRead<Args<'b> = ()>,
{
    fn write_offsets<W: std::io::prelude::Write + Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Write offset data in the order items appear in the binary file.
        self.stream_entries
            .write_offset(writer, base_offset, data_ptr)?;

        let stream_offsets = self.streams.write_offset(writer, base_offset, data_ptr)?;

        self.texture_resources
            .write_offsets(writer, base_offset, data_ptr)?;
        // TODO: Variable padding of 0 or 16 bytes?

        // Write the xbc1 data at the end.
        // This also works for mxmd streams that don't need to write anything.
        for offsets in stream_offsets.0 {
            // The xbc1 offset is relative to the start of the file.
            offsets.write_offsets(writer, 0, data_ptr)?;
        }

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for TextureResourcesOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        if let Some(chr_textures) = &self.chr_textures {
            chr_textures.write_offsets(writer, base_offset, data_ptr)?;
        }
        self.texture_indices
            .write_full(writer, base_offset, data_ptr)?;
        self.low_textures
            .write_full(writer, base_offset, data_ptr)?;

        Ok(())
    }
}
