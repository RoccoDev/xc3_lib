use std::{
    collections::BTreeMap,
    io::{Cursor, Seek, Write},
};

use crate::vertex::AttributeData;
use binrw::{BinResult, BinWrite};
use glam::{Mat4, Vec2, Vec3, Vec4, Vec4Swizzles};
use gltf::{
    buffer::Target,
    json::validation::Checked::{self, Valid},
};

type GltfAttributes = BTreeMap<
    gltf::json::validation::Checked<gltf::Semantic>,
    gltf::json::Index<gltf::json::Accessor>,
>;
type GltfAttribute = (
    gltf::json::validation::Checked<gltf::Semantic>,
    gltf::json::Index<gltf::json::Accessor>,
);

// gltf stores flat lists of attributes and accessors at the root level.
// Create mappings to properly differentiate models and groups.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferKey {
    pub root_index: usize,
    pub group_index: usize,
    pub buffers_index: usize,
    /// Vertex or index buffer index.
    pub buffer_index: usize,
}

// TODO: Use the start index to adjust the buffer offset instead?
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeightGroupKey {
    pub weights_start_index: usize,
    pub flags2: u32,
    pub buffer: BufferKey,
}

// Combined vertex data for a gltf buffer.
#[derive(Default)]
pub struct Buffers {
    pub buffer_bytes: Vec<u8>,
    pub buffer_views: Vec<gltf::json::buffer::View>,
    pub accessors: Vec<gltf::json::Accessor>,

    pub vertex_buffers: BTreeMap<BufferKey, VertexBuffer>,
    pub index_buffer_accessors: BTreeMap<BufferKey, usize>,
    pub weight_groups: BTreeMap<WeightGroupKey, WeightGroup>,
}

// TODO: Also store weights here?
#[derive(Clone)]
pub struct VertexBuffer {
    pub attributes: GltfAttributes,
    pub morph_targets: Vec<GltfAttributes>,
}

pub struct WeightGroup {
    pub weights: GltfAttribute,
    pub indices: GltfAttribute,
}

impl Buffers {
    pub fn insert_vertex_buffer(
        &mut self,
        vertex_buffer: &crate::vertex::VertexBuffer,
        root_index: usize,
        group_index: usize,
        buffers_index: usize,
        buffer_index: usize,
    ) -> BinResult<&VertexBuffer> {
        let key = BufferKey {
            root_index,
            group_index,
            buffers_index,
            buffer_index,
        };
        if !self.vertex_buffers.contains_key(&key) {
            // Assume the base morph target is already applied.
            let attributes = self.write_attributes(&vertex_buffer.attributes)?;

            // Morph targets have their own attribute data.
            let morph_targets = vertex_buffer
                .morph_targets
                .iter()
                .map(|target| {
                    // Convert from a sparse to a dense representation.
                    let vertex_count = vertex_buffer.attributes[0].len();
                    let mut position_deltas = vec![Vec3::ZERO; vertex_count];
                    let mut normal_deltas = vec![Vec3::ZERO; vertex_count];
                    let mut tangent_deltas = vec![Vec3::ZERO; vertex_count];
                    for (i, vertex_index) in target.vertex_indices.iter().enumerate() {
                        position_deltas[*vertex_index as usize] = target.position_deltas[i];
                        normal_deltas[*vertex_index as usize] = target.normal_deltas[i].xyz();
                        tangent_deltas[*vertex_index as usize] = target.tangent_deltas[i].xyz();
                    }

                    // glTF morph targets are defined as a difference with the base target.
                    let mut attributes = attributes.clone();
                    self.insert_positions(&position_deltas, &mut attributes)?;

                    // Normals and tangents also use deltas.
                    // These should use Vec3 to avoid displacing the sign in tangent.w.
                    self.insert_vec3(&normal_deltas, gltf::Semantic::Normals, &mut attributes)?;
                    self.insert_vec3(&tangent_deltas, gltf::Semantic::Tangents, &mut attributes)?;

                    Ok(attributes)
                })
                .collect::<BinResult<Vec<_>>>()?;

            self.vertex_buffers.insert(
                key,
                VertexBuffer {
                    attributes,
                    morph_targets,
                },
            );
        }
        Ok(self.vertex_buffers.get(&key).unwrap())
    }

    pub fn insert_weight_group(
        &mut self,
        buffers: &crate::ModelBuffers,
        skeleton: Option<&crate::Skeleton>,
        key: WeightGroupKey,
    ) -> Option<&WeightGroup> {
        // TODO: rewrite this.
        if !self.weight_groups.contains_key(&key) {
            if let Some(skeleton) = skeleton {
                if let Some(weights) = &buffers.weights {
                    let vertex_buffer = &buffers.vertex_buffers[key.buffer.buffer_index];
                    if let Some(weight_indices) =
                        vertex_buffer.attributes.iter().find_map(|a| match a {
                            AttributeData::WeightIndex(indices) => Some(indices),
                            _ => None,
                        })
                    {
                        let weight_group = self
                            .add_weight_group(
                                skeleton,
                                weights,
                                weight_indices,
                                key.flags2,
                                key.weights_start_index,
                            )
                            .unwrap();
                        self.weight_groups.insert(key, weight_group);
                    }
                }
            }
        }

        self.weight_groups.get(&key)
    }

    fn add_weight_group(
        &mut self,
        skeleton: &crate::Skeleton,
        weights: &crate::skinning::Weights,
        weight_indices: &[[u16; 2]],
        flags2: u32,
        weights_start_index: usize,
    ) -> BinResult<WeightGroup> {
        let skin_weights = weights.weight_buffer(flags2).unwrap();

        // The weights may be defined with a different bone ordering.
        let bone_names: Vec<_> = skeleton.bones.iter().map(|b| b.name.clone()).collect();
        let skin_weights = skin_weights.reindex_bones(bone_names);

        // Each group has a different starting offset.
        // This needs to be applied during reindexing.
        // No offset is needed if no groups are assigned.
        let skin_weights = skin_weights.reindex(weight_indices, weights_start_index as u32);

        let weights_accessor = self.add_values(
            &skin_weights.weights,
            gltf::json::accessor::Type::Vec4,
            gltf::json::accessor::ComponentType::F32,
            Some(Valid(Target::ArrayBuffer)),
            (None, None),
            true,
        )?;
        let indices_accessor = self.add_values(
            &skin_weights.bone_indices,
            gltf::json::accessor::Type::Vec4,
            gltf::json::accessor::ComponentType::U8,
            Some(Valid(Target::ArrayBuffer)),
            (None, None),
            true,
        )?;

        Ok(WeightGroup {
            weights: (Valid(gltf::Semantic::Weights(0)), weights_accessor),
            indices: (Valid(gltf::Semantic::Joints(0)), indices_accessor),
        })
    }

    fn write_attributes(
        &mut self,
        buffer_attributes: &[AttributeData],
    ) -> BinResult<GltfAttributes> {
        let mut attributes = GltfAttributes::new();

        for attribute in buffer_attributes {
            match attribute {
                AttributeData::Position(values) => {
                    self.insert_positions(values, &mut attributes)?;
                }
                AttributeData::Normal(values) => {
                    // Not all applications will normalize the vertex normals.
                    // Use Vec3 instead of Vec4 since it's better supported.
                    let values: Vec<_> = values.iter().map(|v| v.xyz().normalize()).collect();
                    self.insert_vec3(&values, gltf::Semantic::Normals, &mut attributes)?;
                }
                AttributeData::Tangent(values) => {
                    // TODO: do these values need to be scaled/normalized?
                    // TODO: Why is the w component not always 1 or -1?
                    self.insert_vec4(values, gltf::Semantic::Tangents, &mut attributes)?;
                }
                AttributeData::TexCoord0(values) => {
                    self.insert_vec2(values, gltf::Semantic::TexCoords(0), &mut attributes)?;
                }
                AttributeData::TexCoord1(values) => {
                    self.insert_vec2(values, gltf::Semantic::TexCoords(1), &mut attributes)?;
                }
                AttributeData::TexCoord2(values) => {
                    self.insert_vec2(values, gltf::Semantic::TexCoords(2), &mut attributes)?;
                }
                AttributeData::TexCoord3(values) => {
                    self.insert_vec2(values, gltf::Semantic::TexCoords(3), &mut attributes)?;
                }
                AttributeData::TexCoord4(values) => {
                    self.insert_vec2(values, gltf::Semantic::TexCoords(4), &mut attributes)?;
                }
                AttributeData::TexCoord5(values) => {
                    self.insert_vec2(values, gltf::Semantic::TexCoords(5), &mut attributes)?;
                }
                AttributeData::TexCoord6(values) => {
                    self.insert_vec2(values, gltf::Semantic::TexCoords(6), &mut attributes)?;
                }
                AttributeData::TexCoord7(values) => {
                    self.insert_vec2(values, gltf::Semantic::TexCoords(7), &mut attributes)?;
                }
                AttributeData::TexCoord8(values) => {
                    self.insert_vec2(values, gltf::Semantic::TexCoords(8), &mut attributes)?;
                }
                AttributeData::VertexColor(values) => {
                    // TODO: Vertex color isn't always an RGB multiplier?
                    // Use a custom attribute to avoid rendering issues.
                    self.insert_vec4(
                        values,
                        gltf::Semantic::Extras("_Color".to_string()),
                        &mut attributes,
                    )?;
                }
                AttributeData::Blend(values) => {
                    // Used for color blending for some stages.
                    self.insert_vec4(
                        values,
                        gltf::Semantic::Extras("Blend".to_string()),
                        &mut attributes,
                    )?;
                }
                // Skin weights are handled separately.
                AttributeData::WeightIndex(_) => (),
                AttributeData::SkinWeights(_) => (),
                AttributeData::BoneIndices(_) => (),
            }
        }
        Ok(attributes)
    }

    pub fn insert_index_buffer(
        &mut self,
        index_buffer: &crate::vertex::IndexBuffer,
        root_index: usize,
        group_index: usize,
        buffers_index: usize,
        buffer_index: usize,
    ) -> BinResult<usize> {
        let key = BufferKey {
            root_index,
            group_index,
            buffers_index,
            buffer_index,
        };
        if !self.index_buffer_accessors.contains_key(&key) {
            let index_bytes = write_bytes(&index_buffer.indices)?;

            // The offset must be a multiple of the component data type.
            let aligned = self
                .buffer_bytes
                .len()
                .next_multiple_of(std::mem::size_of::<u16>());
            self.buffer_bytes.resize(aligned, 0u8);

            // Assume everything uses the same buffer for now.
            let view = gltf::json::buffer::View {
                buffer: gltf::json::Index::new(0),
                byte_length: index_bytes.len() as u32,
                byte_offset: Some(self.buffer_bytes.len() as u32),
                byte_stride: None,
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                target: Some(Valid(gltf::json::buffer::Target::ElementArrayBuffer)),
            };

            let indices = gltf::json::Accessor {
                buffer_view: Some(gltf::json::Index::new(self.buffer_views.len() as u32)),
                byte_offset: Some(0),
                count: index_buffer.indices.len() as u32,
                component_type: Valid(gltf::json::accessor::GenericComponentType(
                    gltf::json::accessor::ComponentType::U16,
                )),
                extensions: Default::default(),
                extras: Default::default(),
                type_: Valid(gltf::json::accessor::Type::Scalar),
                min: None,
                max: None,
                name: None,
                normalized: false,
                sparse: None,
            };
            self.index_buffer_accessors.insert(
                BufferKey {
                    root_index,
                    group_index,
                    buffers_index,
                    buffer_index,
                },
                self.accessors.len(),
            );

            self.accessors.push(indices);
            self.buffer_views.push(view);
            self.buffer_bytes.extend_from_slice(&index_bytes);
        }

        Ok(*self.index_buffer_accessors.get(&key).unwrap())
    }

    fn insert_positions(
        &mut self,
        values: &[Vec3],
        attributes: &mut GltfAttributes,
    ) -> BinResult<()> {
        // Attributes should be non empty.
        if !values.is_empty() {
            // Only the position attribute requires min/max.
            let min_max = positions_min_max(values);

            let index = self.add_values(
                values,
                gltf::json::accessor::Type::Vec3,
                gltf::json::accessor::ComponentType::F32,
                Some(Valid(Target::ArrayBuffer)),
                min_max,
                true,
            )?;

            // Assume the buffer has only one of each attribute semantic.
            attributes.insert(Valid(gltf::Semantic::Positions), index);
        }
        Ok(())
    }

    fn insert_vec2(
        &mut self,
        values: &[Vec2],
        semantic: gltf::Semantic,
        attributes: &mut GltfAttributes,
    ) -> BinResult<()> {
        self.insert_attribute_values(
            values,
            semantic,
            gltf::json::accessor::Type::Vec2,
            gltf::json::accessor::ComponentType::F32,
            Some(Valid(Target::ArrayBuffer)),
            attributes,
        )
    }

    fn insert_vec3(
        &mut self,
        values: &[Vec3],
        semantic: gltf::Semantic,
        attributes: &mut GltfAttributes,
    ) -> BinResult<()> {
        self.insert_attribute_values(
            values,
            semantic,
            gltf::json::accessor::Type::Vec3,
            gltf::json::accessor::ComponentType::F32,
            Some(Valid(Target::ArrayBuffer)),
            attributes,
        )
    }

    fn insert_vec4(
        &mut self,
        values: &[Vec4],
        semantic: gltf::Semantic,
        attributes: &mut GltfAttributes,
    ) -> BinResult<()> {
        self.insert_attribute_values(
            values,
            semantic,
            gltf::json::accessor::Type::Vec4,
            gltf::json::accessor::ComponentType::F32,
            Some(Valid(Target::ArrayBuffer)),
            attributes,
        )
    }

    fn insert_attribute_values<T: WriteBytes>(
        &mut self,
        values: &[T],
        semantic: gltf::Semantic,
        components: gltf::json::accessor::Type,
        component_type: gltf::json::accessor::ComponentType,
        target: Option<Checked<Target>>,
        attributes: &mut GltfAttributes,
    ) -> BinResult<()> {
        // Attributes should be non empty.
        if !values.is_empty() {
            let index = self.add_values(
                values,
                components,
                component_type,
                target,
                (None, None),
                true,
            )?;

            // Assume the buffer has only one of each attribute semantic.
            attributes.insert(Valid(semantic), index);
        }
        Ok(())
    }

    pub fn add_values<T: WriteBytes>(
        &mut self,
        values: &[T],
        components: gltf::json::accessor::Type,
        component_type: gltf::json::accessor::ComponentType,
        target: Option<Checked<Target>>,
        min_max: (Option<gltf_json::Value>, Option<gltf_json::Value>),
        byte_stride: bool,
    ) -> BinResult<gltf::json::Index<gltf::json::Accessor>> {
        let attribute_bytes = write_bytes(values)?;

        // The offset must be a multiple of the component data type.
        let aligned = self
            .buffer_bytes
            .len()
            .next_multiple_of(std::mem::size_of::<T>());
        self.buffer_bytes.resize(aligned, 0u8);

        // Assume everything uses the same buffer for now.
        // Each attribute is in its own section and thus has its own view.
        let view = gltf::json::buffer::View {
            buffer: gltf::json::Index::new(0),
            byte_length: attribute_bytes.len() as u32,
            byte_offset: Some(self.buffer_bytes.len() as u32),
            byte_stride: byte_stride.then_some(std::mem::size_of::<T>() as u32),
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target,
        };
        self.buffer_bytes.extend_from_slice(&attribute_bytes);

        let (min, max) = min_max;

        let accessor = gltf::json::Accessor {
            buffer_view: Some(gltf::json::Index::new(self.buffer_views.len() as u32)),
            byte_offset: Some(0),
            count: values.len() as u32,
            component_type: Valid(gltf::json::accessor::GenericComponentType(component_type)),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(components),
            min,
            max,
            name: None,
            normalized: false,
            sparse: None,
        };

        let index = gltf::json::Index::new(self.accessors.len() as u32);

        self.accessors.push(accessor);
        self.buffer_views.push(view);

        Ok(index)
    }
}

fn positions_min_max(values: &[Vec3]) -> (Option<gltf_json::Value>, Option<gltf_json::Value>) {
    let min = values.iter().copied().reduce(Vec3::min);
    let max = values.iter().copied().reduce(Vec3::max);

    if let (Some(min), Some(max)) = (min, max) {
        (
            Some(serde_json::json!([min.x, min.y, min.z])),
            Some(serde_json::json!([max.x, max.y, max.z])),
        )
    } else {
        (None, None)
    }
}

// gltf requires little endian for byte buffers.
// Create a trait instead of using bytemuck.
pub trait WriteBytes {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()>;
}

impl WriteBytes for u16 {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.write_le(writer)
    }
}

impl WriteBytes for [u8; 4] {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.write_le(writer)
    }
}

impl WriteBytes for Vec2 {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.to_array().write_le(writer)
    }
}

impl WriteBytes for Vec3 {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.to_array().write_le(writer)
    }
}

impl WriteBytes for Vec4 {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.to_array().write_le(writer)
    }
}

impl WriteBytes for Mat4 {
    fn write<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
        self.to_cols_array().write_le(writer)
    }
}

fn write_bytes<T: WriteBytes>(values: &[T]) -> BinResult<Vec<u8>> {
    let mut writer = Cursor::new(Vec::new());
    for v in values {
        v.write(&mut writer)?;
    }
    Ok(writer.into_inner())
}
