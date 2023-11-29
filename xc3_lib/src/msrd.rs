//! Model resources like shaders, geometry, or textures in `.wismt` files.
//!
//! XC3: `chr/{ch,en,oj,wp}/*.wismt`
use crate::{
    error::DecompressStreamError, mibl::Mibl, mxmd::PackedExternalTextures, parse_count32_offset32,
    parse_opt_ptr32, parse_ptr32, spch::Spch, vertex::VertexData, xbc1::Xbc1,
    xc3_write_binwrite_impl,
};
use bilge::prelude::*;
use binrw::{binread, BinRead, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

/// .wismt model files in `chr/bt`, `chr/ch/`, `chr/en`, `chr/oj`, and `chr/wp`.
#[binread]
#[derive(Debug, Xc3Write)]
#[br(magic(b"DRSM"))]
#[xc3(magic(b"DRSM"))]
pub struct Msrd {
    /// Version `10001`
    pub version: u32,
    // rounded or aligned in some way?
    pub header_size: u32, // TODO: xbc1 offset - 16?
    pub offset: u32,      // TODO: Pointer to an inner type?

    // TODO: variable size?
    pub tag: u32, // 4097?

    pub stream_flags: StreamFlags,

    // TODO: This offset depends on flags like with mxmd models?
    #[br(parse_with = parse_count32_offset32, offset = offset as u64)]
    #[xc3(count_offset(u32, u32))]
    pub stream_entries: Vec<StreamEntry>,

    #[br(parse_with = parse_count32_offset32, offset = offset as u64)]
    #[xc3(count_offset(u32, u32))]
    pub streams: Vec<Stream>,

    // offset 24
    pub vertex_data_entry_index: u32,
    pub shader_entry_index: u32,

    pub low_textures_entry_index: u32,
    pub low_textures_stream_index: u32,

    pub textures_stream_index: u32,
    pub textures_stream_entry_start_index: u32,
    pub textures_stream_entry_count: u32,

    // TODO: identical to indices in mxmd?
    #[br(parse_with = parse_count32_offset32, offset = offset as u64)]
    #[xc3(count_offset(u32, u32))]
    pub texture_indices: Vec<u16>,

    // TODO: Some of these use actual names?
    // TODO: Possible to figure out the hash function used?
    /// Information on the [Mibl] for [low_textures_entry_index](#structfield.low_textures_entry_index).
    /// Identical to the entries in the Mxmd [textures](../mxmd/struct.Mxmd.html#structfield.textures).
    #[br(parse_with = parse_opt_ptr32, offset = offset as u64)]
    #[xc3(offset(u32), align(2))]
    pub low_textures: Option<PackedExternalTextures>,

    pub unk1: u32,

    // TODO: Same count as textures?
    // TODO: This doesn't work for pc000101.wismt?
    #[br(parse_with = parse_count32_offset32, offset = offset as u64)]
    #[xc3(count_offset(u32, u32))]
    pub texture_resources: Vec<TextureResource>,

    // TODO: offset 76?
    // TODO: optional 16 bytes of padding?
    pub unks: [u32; 4],
}

/// A file contained in a [Stream].
#[derive(Debug, BinRead, BinWrite)]
pub struct StreamEntry {
    /// The offset in bytes for the decompressed data range in the stream.
    pub offset: u32,
    /// The size in bytes of the decompressed data range in the stream.
    pub size: u32,
    // TODO: high res mip?
    pub unk_index: u16, // TODO: what does this do?
    pub item_type: EntryType,
    // TODO: padding?
    pub unk: [u32; 2],
}

#[bitsize(32)]
#[derive(DebugBits, FromBits, BinRead, BinWrite, Clone, Copy)]
#[br(map = u32::into)]
#[bw(map = |&x| u32::from(x))]
pub struct StreamFlags {
    pub has_vertex: bool,
    pub has_spch: bool,
    pub has_low_textures: bool,
    pub has_textures: bool,
    pub unk1: u3,
    pub unk: u25,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq)]
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

/// A compressed archive with items determined by [StreamEntry].
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Stream {
    pub comp_size: u32,
    pub decomp_size: u32, // TODO: slightly larger than xbc1 decomp size?
    // TODO: Why does this sometimes have an extra 16 bytes of padding?
    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub xbc1: Xbc1,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct TextureResource {
    // TODO: The the texture name hash as an integer?
    pub hash: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

impl Msrd {
    pub fn decompress_stream(
        &self,
        stream_index: u32,
        entry_index: u32,
    ) -> Result<Vec<u8>, DecompressStreamError> {
        let stream = &self.streams[stream_index as usize].xbc1.decompress()?;
        let entry = &self.stream_entries[entry_index as usize];
        Ok(stream[entry.offset as usize..entry.offset as usize + entry.size as usize].to_vec())
    }

    pub fn extract_vertex_data(&self) -> Result<VertexData, DecompressStreamError> {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.vertex_data_entry_index)?;
        VertexData::from_bytes(bytes).map_err(Into::into)
    }

    pub fn extract_low_textures(&self) -> Result<Vec<Mibl>, DecompressStreamError> {
        let bytes = self.decompress_stream(
            self.low_textures_stream_index,
            self.low_textures_entry_index,
        )?;

        // TODO: Avoid unwrap?
        self.low_textures
            .as_ref()
            .unwrap()
            .textures
            .iter()
            .map(|t| {
                let mibl_bytes =
                    &bytes[t.mibl_offset as usize..t.mibl_offset as usize + t.mibl_length as usize];
                Mibl::from_bytes(mibl_bytes).map_err(Into::into)
            })
            .collect()
    }

    pub fn extract_textures(&self) -> Result<Vec<Mibl>, DecompressStreamError> {
        // The textures are packed into a single stream.
        let stream = &self.streams[self.textures_stream_index as usize]
            .xbc1
            .decompress()?;

        let start = self.textures_stream_entry_start_index as usize;
        let count = self.textures_stream_entry_count as usize;
        self.stream_entries[start..start + count]
            .iter()
            .map(|entry| {
                let bytes =
                    &stream[entry.offset as usize..entry.offset as usize + entry.size as usize];
                Mibl::from_bytes(bytes).map_err(Into::into)
            })
            .collect::<Result<_, _>>()
    }

    pub fn extract_shader_data(&self) -> Result<Spch, DecompressStreamError> {
        // TODO: is this always in the first stream?
        let bytes = self.decompress_stream(0, self.shader_entry_index)?;
        Spch::from_bytes(bytes).map_err(Into::into)
    }
}

xc3_write_binwrite_impl!(StreamEntry, StreamFlags);

impl<'a> Xc3WriteOffsets for MsrdOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
    ) -> xc3_write::Xc3Result<()> {
        // TODO: Rework the msrd types to handle this.
        let base_offset = 16;

        // TODO: find a better way to express variable padding.
        // if self.stream_flags.data.unk1() {
        // *data_ptr += 16;
        // }

        // Write offset data in the order items appear in the binary file.
        self.stream_entries
            .write_offset(writer, base_offset, data_ptr)?;

        let stream_offsets = self.streams.write_offset(writer, base_offset, data_ptr)?;

        // if !self.texture_resources.data.is_empty() {
        self.texture_resources
            .write_offset(writer, base_offset, data_ptr)?;
        // }

        self.texture_indices
            .write_offset(writer, base_offset, data_ptr)?;

        self.low_textures
            .write_full(writer, base_offset, data_ptr)?;

        // TODO: Variable padding?
        // if self.stream_flags.data.unk1().value() == 6 {
        //     *data_ptr += 16;
        // }

        for offsets in stream_offsets.0 {
            offsets.xbc1.write_offset(writer, 0, data_ptr)?;
        }

        Ok(())
    }
}
