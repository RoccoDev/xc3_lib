use crate::{
    msrd::TextureResource, parse_count_offset, parse_offset_count, parse_opt_ptr32, parse_ptr32,
    parse_string_ptr32, spch::Spch, vertex::VertexData,
};
use bilge::prelude::*;
use binrw::{args, binread};
use serde::Serialize;

/// .wimdo files
#[binread]
#[derive(Debug, Serialize)]
#[br(magic(b"DMXM"))]
pub struct Mxmd {
    version: u32,

    // Are the following fields shared with maps?
    #[br(parse_with = parse_ptr32)]
    pub models: Models,

    #[br(parse_with = parse_ptr32)]
    pub materials: Materials,

    #[br(parse_with = parse_opt_ptr32)]
    unk1: Option<Unk1>,

    /// Embedded vertex data for .wimdo only models with no .wismt.
    #[br(parse_with = parse_opt_ptr32)]
    pub vertex_data: Option<VertexData>,

    /// Embedded shader data for .wimdo only models with no .wismt.
    #[br(parse_with = parse_opt_ptr32)]
    pub spch: Option<Spch>,

    unk4: u32,
    unk5: u32,

    // unpacked textures?
    #[br(parse_with = parse_ptr32)]
    pub textures: Textures,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Materials {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_offset_count, args { offset: base_offset, inner: base_offset })]
    pub materials: Vec<Material>,

    // offset?
    unk1: u32,
    unk2: u32,

    // TODO: Materials have offsets into these arrays for parameter values?
    // material body has a uniform at shader offset 64 but offset 48 in this floats buffer
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    floats: Vec<f32>,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    ints: Vec<u32>,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    unk_offset1: MaterialUnk1,

    // TODO: is this ever not 0?
    unk4: u32,

    #[br(parse_with = parse_offset_count, args { offset: base_offset, inner: base_offset })]
    unks: Vec<MaterialUnk>,

    unks1: [u32; 2],

    #[br(parse_with = parse_count_offset, offset = base_offset)]
    unks2: Vec<(u32, u32)>,

    unks3: [u32; 7],

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    pub samplers: Option<Samplers>,

    // TODO: padding?
    unks4: [u32; 4],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk {
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    unk1: Vec<(u32, u32)>,

    unk3: u32, // 0
    unk4: u32, // 0

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    unk5: Vec<[u32; 6]>,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    unk7: Vec<u16>,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    unk9: Vec<(u16, u16)>,

    unk11: u32,
    unk12: u16, // counts up from 0?
    unk13: u16, // unk11 + unk12?

    // TODO: padding?
    padding: [u32; 5],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct MaterialUnk1 {
    // count matches up with Material.unk_start_index?
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    unk1: Vec<(u16, u16)>,
    // 0 1 2 ... count-1
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    unk2: Vec<u16>,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Samplers {
    unk1: u32, // count?
    unk2: u32, // offset?
    unk3: u32, // pad?
    unk4: u32, // pad?

    // pointed to by above?
    #[br(count = unk1)]
    pub samplers: Vec<Sampler>,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Sampler {
    // TODO: Serialize bitfields like structs?
    #[br(map(|x: u32| x.into()))]
    #[serde(skip_serializing)]
    pub flags: SamplerFlags,

    // Is this actually a float?
    pub unk2: f32,
}

/// Texture sampler settings for addressing and filtering.
#[bitsize(32)]
#[derive(DebugBits, FromBits, Clone, Copy)]
pub struct SamplerFlags {
    /// Sets wrap U to repeat when `true`.
    pub repeat_u: bool,
    /// Sets wrap V to repeat when `true`.
    pub repeat_v: bool,
    /// Sets wrap U to mirrored repeat when `true` regardless of repeat U.
    pub mirror_u: bool,
    /// Sets wrap V to mirrored repeat when `true` regardless of repeat V.
    pub mirror_v: bool,
    /// Sets min and mag filter to nearest when `true`.
    /// The min filter also depends on disable_mipmap_filter.
    pub nearest: bool,
    /// Sets all wrap modes to clamp and min and mag filter to linear.
    /// Ignores the values of previous flags.
    pub force_clamp: bool,
    /// Removes the mipmap nearest from the min filter when `true`.
    pub disable_mipmap_filter: bool,
    unk1: bool,
    unk3: bool,
    unk: u23,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct Material {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub name: String,

    unk1: u16,
    unk2: u16,
    unk3: u16,
    unk4: u16,

    /// Color multiplier value assigned to the `gMatCol` shader uniform.
    pub color: [f32; 4],

    unk_float: f32,

    // TODO: materials with zero textures?
    /// Defines the shader's sampler bindings in order for s0, s1, s2, ...
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub textures: Vec<Texture>,

    pub flags: MaterialFlags,

    // Parameters?
    m_unks1_1: u32,
    m_unks1_2: u32,
    m_unks1_3: u32,
    m_unks1_4: u32,
    floats_start_index: u32,
    ints_start_index: u32,
    ints_count: u32,

    // always count 1?
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub shader_programs: Vec<ShaderProgram>,

    unk5: u32,

    // index for MaterialUnk1.unk1?
    unk_start_index: u16, // sum of previous unk_count?
    unk_count: u16,

    m_unks2: [u16; 12],
}

#[binread]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct MaterialFlags {
    pub flag0: u8,
    pub blend_state: BlendState,
    pub cull_mode: CullMode,
    pub flag3: u8,
    pub stencil_state1: StencilState1,
    pub stencil_state2: StencilState2,
    pub depth_func: DepthFunc,
    pub flag7: u8,
}

// TODO: Convert these to equations for RGB and alpha for docs.
// TODO: Is it worth documenting this outside of xc3_wgpu?
// flag, col src, col dst, col op, alpha src, alpha dst, alpha op
// 0 = disabled
// 1, Src Alpha, 1 - Src Alpha, Add, Src Alpha, 1 - Src Alpha, Add
// 2, Src Alpha, One, Add, Src Alpha, One, Add
// 3, Zero, Src Col, Add, Zero, Src Col, Add
// 6, disabled + ???
#[binread]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[br(repr(u8))]
pub enum BlendState {
    Disabled = 0,
    AlphaBlend = 1,
    Additive = 2,
    Multiplicative = 3,
    Unk6 = 6, // also disabled?
}

// TODO: Get the actual stencil state from RenderDoc.
// 0 = disables hair blur stencil stuff?
// 4 = disables hair but different ref value?
// 16 = enables hair blur stencil stuff?
#[binread]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[br(repr(u8))]
pub enum StencilState1 {
    Always = 0,
    Unk1 = 1,
    Always2 = 4,
    Unk5 = 5,
    Unk8 = 8,
    Unk9 = 9,
    UnkHair = 16,
    Unk20 = 20,
}

// TODO: Does this flag actually disable stencil?
#[binread]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[br(repr(u8))]
pub enum StencilState2 {
    Disabled = 0,
    Enabled = 1,
    Unk2 = 2,
    Unk6 = 6,
    Unk7 = 7,
    Unk8 = 8,
}

#[binread]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[br(repr(u8))]
pub enum DepthFunc {
    Disabled = 0,
    LessEqual = 1,
    Equal = 3,
}

#[binread]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[br(repr(u8))]
pub enum CullMode {
    Back = 0,
    Front = 1,
    Disabled = 2,
    Unk3 = 3, // front + ???
}

#[binread]
#[derive(Debug, Serialize)]
pub struct ShaderProgram {
    pub program_index: u32, // index into programs in wismt?
    pub unk_type: ShaderUnkType,
    pub parent_material_index: u16, // index of the parent material?
    pub unk4: u32,                  // always 1?
}

// Affects what pass the object renders in?
// Each "pass" has different render targets?
// _trans = 1,
// _ope = 0,1,7
// _zpre = 0
// _outline = 0
#[binread]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize)]
#[br(repr(u16))]
pub enum ShaderUnkType {
    Unk0 = 0, // main opaque + some transparent?
    Unk1 = 1, // second layer transparent?
    Unk6 = 6, // used for maps?
    Unk7 = 7, // additional eye effect layer?
    Unk9 = 9, // used for maps?
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Texture {
    pub texture_index: u16,
    pub sampler_index: u16,
    pub unk2: u16,
    pub unk3: u16,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Models {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    unk1: u32,

    max_xyz: [f32; 3],
    min_xyz: [f32; 3],

    #[br(parse_with = parse_offset_count, args { offset: base_offset, inner: base_offset })]
    pub models: Vec<Model>,

    unk2: u32,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    skeleton: Option<Skeleton>,

    unks3: [u32; 22],

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    pub unk_offset1: Option<MeshUnk1>,

    unk_offset2: u32,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    lod_data: Option<LodData>,
}

/// A collection of meshes where each [Mesh] represents one draw call.
///
/// Each [Model] has an associated [VertexData](crate::vertex::VertexData) containing vertex and index buffers.
#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct Model {
    #[br(parse_with = parse_offset_count, offset = base_offset)]
    pub meshes: Vec<Mesh>,

    unk1: u32,
    max_xyz: [f32; 3],
    min_xyz: [f32; 3],
    bounding_radius: f32,
    unks: [u32; 7],
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Mesh {
    flags1: u32,
    flags2: u32,
    pub vertex_buffer_index: u16,
    pub index_buffer_index: u16,
    unk_index: u16,
    pub material_index: u16,
    unk2: u32,
    unk3: u32,
    unk4: u32,
    unk5: u16,
    pub lod: u16,
    // TODO: groups?
    unks6: [i32; 4],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct MeshUnk1 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    pub inner: MeshUnk1Inner,
    unk1: [u32; 14],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct MeshUnk1Inner {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub unk1: String,

    unk2: [f32; 9],
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct LodData {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    unk1: u32,

    // another list?
    unk2: u32,
    unk3: u32,

    #[br(parse_with = parse_offset_count, offset = base_offset)]
    items: Vec<(u16, u16)>,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Textures {
    // TODO: The fields change depending on some sort of flag?
    tag: u32, // 4097 or sometimes 0?

    #[br(args_raw(tag))]
    pub inner: TexturesInner,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(tag: u32))]
pub enum TexturesInner {
    #[br(pre_assert(tag == 0))]
    Unk0(Textures1),
    #[br(pre_assert(tag == 4097))]
    Unk1(Textures2),
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Textures1 {
    // Subtract the tag size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    unk1: u32, // TODO: count for multiple packed textures?
    // low textures?
    #[br(parse_with = parse_ptr32, offset = base_offset)]
    pub textures1: PackedTextures,
    // high textures?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    pub textures2: Option<PackedTextures>,

    unk4: u32,
    unk5: u32,
    // TODO: more fields?
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Textures2 {
    // Subtract the tag size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    unk2: u32, // 103

    // TODO: count offset?
    unk3: u32,
    unk4: u32,

    // TODO: count?
    unk5: u32,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    unk_offset: TexturesUnk,

    unks2: [u32; 7],

    #[br(parse_with = parse_count_offset, offset = base_offset)]
    indices: Vec<u16>,

    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    pub items: Option<PackedTextures>,

    unk7: u32,

    // TODO: same as the type in msrd?
    #[br(parse_with = parse_count_offset, offset = base_offset)]
    resources: Vec<TextureResource>,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct TexturesUnk {
    unk1: u32,
    unk2: u32,
    unk3: u32,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct PackedTextures {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count_offset, args { offset: base_offset, inner: base_offset })]
    pub textures: Vec<PackedTexture>,

    unk2: u32,
    strings_offset: u32,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct PackedTexture {
    unk1: u32,

    // TODO: These offsets are for different places for maps and characters?
    pub mibl_length: u32,
    pub mibl_offset: u32,

    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    pub name: String,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Skeleton {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    count1: u32,
    count2: u32,

    // TODO: Find a simpler way of writing this?
    #[br(parse_with = parse_ptr32)]
    #[br(args {
        offset: base_offset,
        inner: args! {
            count: count1 as usize,
            inner: base_offset
        }
    })]
    bones: Vec<Bone>,

    // TODO: Create a matrix type?
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: count1 as usize } })]
    transforms: Vec<[[f32; 4]; 4]>,

    unk_offset1: u32,
    unk_offset2: u32,
    count3: u32,
    unk_offset3: u32,
    unk_offset4: u32,
}

#[binread]
#[derive(Debug, Serialize)]
#[br(import_raw(base_offset: u64))]
pub struct Bone {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    name: String,
    unk1: f32,
    unk_type: u32,
    #[br(pad_after = 8)]
    unk_index: u32,
}

// TODO: pointer to decl_gbl_cac in ch001011011.wimdo?
#[binread]
#[derive(Debug, Serialize)]
#[br(stream = r)]
pub struct Unk1 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    #[br(parse_with = parse_count_offset, offset = base_offset)]
    unk1: Vec<Unk1Unk1>,

    #[br(parse_with = parse_count_offset, offset = base_offset)]
    unk2: Vec<Unk1Unk2>,

    #[br(parse_with = parse_count_offset, offset = base_offset)]
    unk3: Vec<Unk1Unk3>,

    // angle values?
    #[br(parse_with = parse_count_offset, offset = base_offset)]
    unk4: Vec<Unk1Unk4>,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Unk1Unk1 {
    index: u16,
    unk2: u16, // 1
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Unk1Unk2 {
    unk1: u16, // 0
    index: u16,
    unk3: u16,
    unk4: u16,
    unk5: u32, // 0
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Unk1Unk3 {
    unk1: u16,
    unk2: u16,
    unk3: u32,
    unk4: u16,
    unk5: u16,
    unk6: u16,
    unk7: u16,
}

#[binread]
#[derive(Debug, Serialize)]
pub struct Unk1Unk4 {
    unk1: f32,
    unk2: f32,
    unk3: f32,
    unk4: u32,
}
