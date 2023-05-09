use std::{
    io::SeekFrom,
    path::{Path, PathBuf},
};

use crate::parse_string_ptr;
use binrw::{
    args, binread, helpers::count_with, BinRead, BinResult, FilePtr32, FilePtr64, NullString,
};
use serde::Serialize;

// embedded in .wismt files
// TODO: also .wishp files?
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"HCPS"))]
pub struct Hpcs {
    version: u32,
    unk1: u32,
    count: u32,
    #[br(parse_with = FilePtr32::parse)]
    string_section: StringSection,
    unk4: u32,

    #[br(temp)]
    slct_base_offset: u32,

    unk6: u32,

    // Compiled shader binaries.
    // Alternates between vertex and fragment shaders.
    xv4_base_offset: u32,
    xv4_section_length: u32,

    // data before the xV4 section
    // same count as xV4 but with magic 0x34127698?
    // each has length 2176 (referenced in shaders?)
    unk_section_offset: u32,
    unk_section_length: u32,

    #[br(pad_after = 20)]
    unk11: u32,

    #[br(count = count)]
    #[br(args {
        inner: args! {
            slct_base_offset: slct_base_offset as u64,
            unk_base_offset: unk_section_offset as u64,
        }
    })]
    shader_programs: Vec<ShaderProgram>,
}

#[binread]
#[derive(Debug, Serialize)]
struct StringSection {
    #[br(temp)]
    count: u32, // program count?
    unk12: u32,
    unk13: u32,
    #[br(parse_with = count_with(count as usize, parse_string_ptr))]
    program_names: Vec<String>,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import { slct_base_offset: u64, unk_base_offset: u64 })]
pub struct ShaderProgram {
    #[br(parse_with = FilePtr64::parse)]
    #[br(args { offset: slct_base_offset, inner: args! { unk_base_offset } })]
    slct: Slct,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"SLCT"))]
#[br(import { unk_base_offset: u64, })]
#[br(stream = r)]
pub struct Slct {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    unk1: u32,

    #[br(temp)]
    unk_strings_count: u32,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { 
        offset: base_offset, 
        inner: args! { 
            count: unk_strings_count as usize,
            inner: args! { base_offset }
        }
    })]
    unk_strings: Vec<UnkString>,

    #[br(temp)]
    unk4_count: u32,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { offset: base_offset, inner: args! { count: unk4_count as usize }})]
    unk4: Vec<(u32, u32)>,

    unk6: u32,

    unk_offset: u32,

    // this is actually the inner offset and the string base offset?
    #[br(parse_with = FilePtr32::parse, offset = base_offset)]
    inner: SlctInner, // base offset for strings relative to start of slct?

    unk_offset1: u32,

    #[br(parse_with = FilePtr32::parse, offset = unk_base_offset)]
    unk_item: UnkItem,

    unk_offset2: u32,

    vertex_xv4_offset: u32, // relative to xv4 base offset
    xv4_total_size: u32,    // size of vertex + fragment?

    unks1: [u32; 4],
    // end of slct main header?
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import { base_offset: u64 })]
struct UnkString {
    unk1: u32,
    unk2: u32,
    #[br(parse_with = parse_string_ptr, args(base_offset))]
    text: String
}

// always 112 bytes?
#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
struct SlctInner {
    #[br(try_calc = r.stream_position())]
    base_offset: u64,

    unks2: [u32; 8],

    unk_count1: u16,
    unk_count2: u16,

    unk12: u32,
    unk13: u32,

    unk_count3: u16,
    unk_count4: u16,

    unk14: u32,
    unk15: u32,

    unk_count5: u16,
    unk_count6: u16,

    unks2_1: [u32; 5],

    attribute_count: u32,
    unk9: u32,
    uniform_count: u32,
    unk11: u32,
    unks3: [u32; 4],

    #[br(args {
        string_offset: base_offset as u64,
        attribute_count: attribute_count as usize,
        uniform_count: uniform_count as usize,
        // TODO: Why are there multiple count values?
        // TODO: fragment + vertex counts?
        buffer_count: unk_count1 as usize + unk_count3 as usize,
        sampler_count: unk_count5 as usize
    })]
    nvsd: Nvsd,
}

#[binread]
#[derive(Debug, Serialize)]
struct UnkItem {
    unk: [u32; 9],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"NVSD"))]
#[br(import {
    string_offset: u64,
    attribute_count: usize,
    uniform_count: usize,
    buffer_count: usize,
    sampler_count: usize,
})]
pub struct Nvsd {
    version: u32,
    unk1: u32, // 0
    unk2: u32, // 0
    unk3: u32, // identical to vertex_xv4_size?
    unk4: u32, // 0
    unk5: u32, // identical to unk_size1?
    unk6: u32, // 1

    // TODO: xV4 header should be stripped when decompiling?
    /// The size of the vertex shader pointed to by the [Slct].
    vertex_xv4_size: u32,
    /// The size of the fragment shader pointed to by the [Slct].
    fragment_xv4_size: u32,

    // Corresponding unk entry size for the two shaders?
    unk_size1: u32, // 2176
    unk_size2: u32, // 2176

    #[br(args { count: buffer_count, inner: args! { string_offset } })]
    buffers: Vec<UniformBuffer>,

    #[br(args { count: sampler_count, inner: args! { string_offset } })]
    samplers: Vec<Sampler>,

    #[br(args { count: attribute_count, inner: args! { string_offset } })]
    attributes: Vec<InputAttribute>,

    #[br(args { count: uniform_count, inner: args! { string_offset } })]
    uniforms: Vec<Uniform>,

    // TODO: What controls this count?
    unks4: [u16; 8],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import { string_offset: u64 })]
struct UniformBuffer {
    #[br(parse_with = parse_string_ptr, args(string_offset))]
    name: String,
    uniform_count: u16,
    uniform_start_index: u16,
    unk3: u32, // 470 + handle * 2?
    unk4: u16,
    unk5: u16,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import { string_offset: u64 })]
struct Sampler {
    #[br(parse_with = parse_string_ptr, args(string_offset))]
    name: String,
    unk1: u32,
    unk2: u32, // handle = (unk2 - 256) * 2 + 8?
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import { string_offset: u64 })]
struct Uniform {
    #[br(parse_with = parse_string_ptr, args(string_offset))]
    name: String,
    unk1: u32,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import { string_offset: u64 })]
struct InputAttribute {
    #[br(parse_with = parse_string_ptr, args(string_offset))]
    name: String,
    location: u32,
}

fn parse_unk_str<R: std::io::Read + std::io::Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: (u64,),
) -> BinResult<Option<String>> {
    let start_offset = u32::read_options(reader, endian, ())?;
    let end_offset = u32::read_options(reader, endian, ())?;

    if start_offset > 0 && end_offset > 0 {
        reader.seek(SeekFrom::Start(args.0 + start_offset as u64))?;

        let value = NullString::read_options(reader, endian, ())?;

        reader.seek(SeekFrom::Start(args.0 + end_offset as u64))?;
        Ok(Some(value.to_string()))
    } else {
        Ok(None)
    }
}

pub fn extract_shader_binaries<P: AsRef<Path>>(
    hpcs: &Hpcs,
    file_data: &[u8],
    output_folder: P,
    ryujinx_shader_tools: Option<String>, // TODO: make this generic?
) {
    for (program, name) in hpcs
        .shader_programs
        .iter()
        .zip(&hpcs.string_section.program_names)
    {
        let base = hpcs.xv4_base_offset as usize + program.slct.vertex_xv4_offset as usize;

        // The first offset is the vertex shader.
        let vert_base = base;
        let vert_size = program.slct.inner.nvsd.vertex_xv4_size as usize;
        // Strip the xV4 header for easier decompilation.
        let vertex = &file_data[vert_base..vert_base + vert_size][48..];

        let vert_file = output_folder.as_ref().join(&format!("{name}_VS.bin"));
        std::fs::write(&vert_file, vertex).unwrap();

        // The fragment shader immediately follows the vertex shader.
        let frag_base = base + vert_size;
        let frag_size = program.slct.inner.nvsd.fragment_xv4_size as usize;
        let fragment = &file_data[frag_base..frag_base + frag_size][48..];

        let frag_file = output_folder.as_ref().join(&format!("{name}_FS.bin"));
        std::fs::write(&frag_file, fragment).unwrap();

        // Decompile using Ryujinx.ShaderTools.exe.
        // There isn't Rust code for this, so just take an exe path.
        if let Some(shader_tools) = &ryujinx_shader_tools {
            std::process::Command::new(shader_tools)
                .args([&vert_file, &vert_file.with_extension("glsl")])
                .output()
                .unwrap();

            std::process::Command::new(shader_tools)
                .args([&frag_file, &frag_file.with_extension("glsl")])
                .output()
                .unwrap();
        }
    }
}
