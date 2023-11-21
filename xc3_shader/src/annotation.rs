use std::collections::HashMap;

use glsl_lang::{
    ast::{
        ArraySpecifier, ArraySpecifierData, ArraySpecifierDimensionData, ArrayedIdentifierData,
        Block, Expr, ExprData, Identifier, Node, StructFieldSpecifierData, TranslationUnit,
        TypeSpecifierData, TypeSpecifierNonArrayData,
    },
    parse::DefaultParse,
    transpiler::glsl::{show_translation_unit, FormattingState},
    visitor::{HostMut, Visit, VisitorMut},
};
use xc3_lib::spch::Nvsd;

// TODO: A more reliable way to do replacement is to visit each identifier.
// Names should be replaced using a lookup table in a single pass.
// String replacement won't handle the case where names overlap.
// TODO: What is the performance cost of annotation?
const VEC4_SIZE: u32 = 16;

struct Annotator {
    replacements: HashMap<String, String>,
    struct_fields: HashMap<String, Vec<Field>>,
}

struct Field {
    name: String,
    // Index of the start of this field.
    vec4_index: u32,
    ty: TypeSpecifierNonArrayData,
    array_length: Option<u32>,
}

// TODO: Clean up usage of AST.
impl VisitorMut for Annotator {
    fn visit_identifier(&mut self, ident: &mut Identifier) -> Visit {
        if let Some(name) = self.replacements.get(ident.as_str()) {
            ident.0 = name.into();
        }
        Visit::Children
    }

    fn visit_block(&mut self, block: &mut Block) -> Visit {
        if let Some(fields) = block
            .identifier
            .as_ref()
            .map(|ident| &ident.ident.0)
            .and_then(|i| self.struct_fields.get(i.as_str()))
        {
            if !fields.is_empty() {
                block.fields = fields.iter().map(field).collect();
            }
        }

        Visit::Children
    }

    fn visit_expr(&mut self, expr: &mut Expr) -> Visit {
        if let ExprData::Bracket(var, specifier) = &mut expr.content {
            if let ExprData::IntConst(index) = &mut specifier.content {
                match &mut var.content {
                    ExprData::Variable(_id) => {
                        // buffer[index].x
                        // TODO: How to handle this case?
                    }
                    ExprData::Dot(e, _field) => {
                        if let ExprData::Variable(id) = &e.content {
                            // buffer.field[index].x
                            if let Some(buffer_name) = self.replacements.get(id.as_str()) {
                                if let Some(fields) = self.struct_fields.get(id.as_str()) {
                                    if let Some((uniform, array_index)) =
                                        find_field(fields, *index as u32)
                                    {
                                        // Assume the field is always "data" for now to match Ryujinx.
                                        let variable = ExprData::Variable(Identifier::new(
                                            buffer_name.as_str().into(),
                                            None,
                                        ));

                                        // buffer.uniform
                                        let new_expr = Expr::new(
                                            ExprData::Dot(
                                                Box::new(Expr::new(variable, None)),
                                                Identifier::new(uniform.as_str().into(), None),
                                            ),
                                            None,
                                        );

                                        *expr = match array_index {
                                            // buffer.uniform[array_index].x
                                            Some(array_index) => Expr::new(
                                                ExprData::Bracket(
                                                    Box::new(new_expr),
                                                    Box::new(Node::new(
                                                        ExprData::IntConst(array_index as i32),
                                                        None,
                                                    )),
                                                ),
                                                None,
                                            ),
                                            // buffer.uniform.x
                                            None => new_expr,
                                        };
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        Visit::Children
    }
}

fn find_field(fields: &[Field], vec4_index: u32) -> Option<(&String, Option<u32>)> {
    fields.iter().find_map(|f| {
        match f.array_length {
            Some(length) => {
                // Check if the vec4 index falls within this array field.
                if vec4_index - f.vec4_index < length {
                    Some((&f.name, Some(vec4_index - f.vec4_index)))
                } else {
                    None
                }
            }
            None => {
                if f.vec4_index == vec4_index {
                    Some((&f.name, None))
                } else {
                    None
                }
            }
        }
    })
}

fn field(field: &Field) -> Node<StructFieldSpecifierData> {
    Node::new(
        StructFieldSpecifierData {
            qualifier: None,
            ty: Node::new(
                TypeSpecifierData {
                    ty: Node::new(field.ty.clone(), None),
                    array_specifier: None,
                },
                None,
            ),
            identifiers: vec![Node::new(
                ArrayedIdentifierData {
                    ident: Identifier::new(field.name.as_str().into(), None),
                    array_spec: field.array_length.map(|i| {
                        ArraySpecifier::new(
                            ArraySpecifierData {
                                dimensions: vec![Node::new(
                                    ArraySpecifierDimensionData::ExplicitlySized(Box::new(
                                        Node::new(ExprData::IntConst(i as i32), None),
                                    )),
                                    None,
                                )],
                            },
                            None,
                        )
                    }),
                },
                None,
            )],
        },
        None,
    )
}

pub fn annotate_fragment(glsl: String, metadata: &Nvsd) -> String {
    let mut replacements = HashMap::new();
    let mut struct_fields = HashMap::new();

    annotate_samplers(&mut replacements, metadata);
    annotate_buffers(&mut replacements, &mut struct_fields, "fp", metadata);

    let mut visitor = Annotator {
        replacements,
        struct_fields,
    };

    let modified_source = shader_source_no_extensions(glsl);
    let mut translation_unit = TranslationUnit::parse(&modified_source).unwrap();
    translation_unit.visit_mut(&mut visitor);

    let mut text = String::new();
    show_translation_unit(&mut text, &translation_unit, FormattingState::default()).unwrap();

    text
}

fn annotate_samplers(replacements: &mut HashMap<String, String>, metadata: &Nvsd) {
    if let Some(samplers) = &metadata.samplers {
        for sampler in samplers {
            let handle = sampler.handle.handle * 2 + 8;
            let texture_name = format!("fp_t_tcb_{handle:X}");
            replacements.insert(texture_name, sampler.name.clone());
        }
    }
}

pub fn annotate_vertex(glsl: String, metadata: &Nvsd) -> String {
    let mut replacements = HashMap::new();
    let mut struct_fields = HashMap::new();

    for attribute in &metadata.attributes {
        let attribute_name = format!("in_attr{}", attribute.location);
        replacements.insert(attribute_name, attribute.name.clone());
    }
    annotate_buffers(&mut replacements, &mut struct_fields, "vp", metadata);

    let mut visitor = Annotator {
        replacements,
        struct_fields,
    };

    // TODO: Find a better way to skip unsupported extensions.
    let modified_source = shader_source_no_extensions(glsl);
    let mut translation_unit = TranslationUnit::parse(&modified_source).unwrap();
    translation_unit.visit_mut(&mut visitor);

    let mut text = String::new();
    show_translation_unit(&mut text, &translation_unit, FormattingState::default()).unwrap();

    text
}

fn annotate_buffers(
    replacements: &mut HashMap<String, String>,
    struct_fields: &mut HashMap<String, Vec<Field>>,
    prefix: &str,
    metadata: &Nvsd,
) {
    // TODO: annotate constants from fp_v1 or vp_c1.
    // TODO: How to determine which constant elements are actually used?
    // TODO: are all uniforms vec4 params?
    // TODO: add initialization code so that annotated shaders still compile.
    if let Some(uniform_buffers) = &metadata.uniform_buffers {
        for buffer in uniform_buffers {
            // TODO: why is this always off by 3?
            // TODO: Is there an fp_c2?
            let handle = buffer.handle.handle + 3;

            let buffer_name = format!("{prefix}_c{handle}");
            let buffer_name_prefixed = format!("_{prefix}_c{handle}");

            replacements.insert(buffer_name.clone(), buffer.name.clone());
            replacements.insert(buffer_name_prefixed.clone(), format!("_{}", buffer.name));

            let start = buffer.uniform_start_index as usize;
            let count = buffer.uniform_count as usize;

            // Sort to make it easier to convert offsets to sizes.
            let mut uniforms = metadata.uniforms[start..start + count].to_vec();
            uniforms.sort_by_key(|u| u.buffer_offset);

            for (uniform_index, uniform) in uniforms.iter().enumerate() {
                let vec4_index = uniform.buffer_offset / VEC4_SIZE;

                // "array[0]" -> "array"
                let uniform_name = uniform
                    .name
                    .find('[')
                    .map(|bracket_index| uniform.name[..bracket_index].to_string())
                    .unwrap_or_else(|| uniform.name.to_string());

                // The array has elements until the next uniform.
                // All uniforms are vec4, so we don't need to worry about std140 alignment.
                // Treat matrix types as vec4 arrays for now to match the decompiled code.
                let array_length = uniforms.get(uniform_index + 1).and_then(|u| {
                    let length = (u.buffer_offset - uniform.buffer_offset) / VEC4_SIZE;
                    if length > 1 {
                        Some(length)
                    } else {
                        // TODO: Infer the length from the highest accessed index?
                        None
                    }
                });

                if let Some(array_length) = array_length {
                    // Annotate all elments from array[0] to array[length-1].
                    // This avoids unannotated entries in the gbuffer database.
                    for i in 0..array_length {
                        let pattern = format!("{}.data[{}]", buffer.name, vec4_index + i);
                        // Reindex the array starting from the base offset.
                        let uniform_name = format!("{}_{}[{i}]", buffer.name, &uniform_name);
                        replacements.insert(pattern, uniform_name);
                    }
                }

                // Add a single field to the uniform buffer.
                // All uniforms are vec4, so we don't need to worry about std140 alignment.
                struct_fields
                    .entry(buffer_name.clone())
                    .and_modify(|e| {
                        e.push(Field {
                            name: uniform_name.clone(),
                            vec4_index,
                            ty: TypeSpecifierNonArrayData::Vec4,
                            array_length,
                        })
                    })
                    .or_default();
            }
        }
    }

    if let Some(storage_buffers) = &metadata.storage_buffers {
        for buffer in storage_buffers {
            let handle = buffer.handle.handle;
            replacements.insert(format!("{prefix}_s{handle}"), buffer.name.clone());
            replacements.insert(format!("_{prefix}_s{handle}"), format!("_{}", buffer.name));
        }
    }
}

fn shader_source_no_extensions(glsl: String) -> String {
    // TODO: Find a better way to skip unsupported extensions.
    glsl.find("#pragma")
        .map(|i| glsl[i..].to_string())
        .unwrap_or(glsl)
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use xc3_lib::spch::{Handle, InputAttribute, Sampler, Uniform, UniformBuffer, Visibility};

    fn metadata() -> Nvsd {
        Nvsd {
            uniform_buffers: Some(vec![
                UniformBuffer {
                    name: "U_CamoflageCalc".to_string(),
                    uniform_count: 1,
                    uniform_start_index: 0,
                    unk3: 672,
                    handle: Handle {
                        handle: 5,
                        visibility: Visibility::VertexFragment,
                    },
                    unk5: 224,
                },
                UniformBuffer {
                    name: "U_Mate".to_string(),
                    uniform_count: 3,
                    uniform_start_index: 1,
                    unk3: 676,
                    handle: Handle {
                        handle: 1,
                        visibility: Visibility::VertexFragment,
                    },
                    unk5: 96,
                },
                UniformBuffer {
                    name: "U_Mdl".to_string(),
                    uniform_count: 4,
                    uniform_start_index: 4,
                    unk3: 680,
                    handle: Handle {
                        handle: 2,
                        visibility: Visibility::Fragment,
                    },
                    unk5: 176,
                },
                UniformBuffer {
                    name: "U_RimBloomCalc".to_string(),
                    uniform_count: 1,
                    uniform_start_index: 8,
                    unk3: 682,
                    handle: Handle {
                        handle: 4,
                        visibility: Visibility::Fragment,
                    },
                    unk5: 32,
                },
                UniformBuffer {
                    name: "U_Static".to_string(),
                    uniform_count: 18,
                    uniform_start_index: 9,
                    unk3: 684,
                    handle: Handle {
                        handle: 0,
                        visibility: Visibility::VertexFragment,
                    },
                    unk5: 672,
                },
                UniformBuffer {
                    name: "U_VolTexCalc".to_string(),
                    uniform_count: 1,
                    uniform_start_index: 27,
                    unk3: 688,
                    handle: Handle {
                        handle: 3,
                        visibility: Visibility::Fragment,
                    },
                    unk5: 176,
                },
            ]),
            storage_buffers: Some(vec![
                UniformBuffer {
                    name: "U_Bone".to_string(),
                    uniform_count: 1,
                    uniform_start_index: 28,
                    unk3: 690,
                    handle: Handle {
                        handle: 0,
                        visibility: Visibility::Fragment,
                    },
                    unk5: 48,
                },
                UniformBuffer {
                    name: "U_OdB".to_string(),
                    uniform_count: 1,
                    uniform_start_index: 29,
                    unk3: 692,
                    handle: Handle {
                        handle: 1,
                        visibility: Visibility::Fragment,
                    },
                    unk5: 48,
                },
            ]),
            attributes: vec![
                InputAttribute {
                    name: "nWgtIdx".to_string(),
                    location: 1,
                },
                InputAttribute {
                    name: "vColor".to_string(),
                    location: 3,
                },
                InputAttribute {
                    name: "vNormal".to_string(),
                    location: 4,
                },
                InputAttribute {
                    name: "vPos".to_string(),
                    location: 0,
                },
                InputAttribute {
                    name: "vTan".to_string(),
                    location: 5,
                },
                InputAttribute {
                    name: "vTex0".to_string(),
                    location: 2,
                },
            ],
            uniforms: vec![
                Uniform {
                    name: "gCamouflageCalcWork[0]".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gTexMat".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gWrkCol".to_string(),
                    buffer_offset: 80,
                },
                Uniform {
                    name: "gWrkFl4[0]".to_string(),
                    buffer_offset: 32,
                },
                Uniform {
                    name: "gMdlParm".to_string(),
                    buffer_offset: 160,
                },
                Uniform {
                    name: "gmWVP".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gmWorld".to_string(),
                    buffer_offset: 64,
                },
                Uniform {
                    name: "gmWorldView".to_string(),
                    buffer_offset: 112,
                },
                Uniform {
                    name: "gRimBloomCalcWork[0]".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gBilMat".to_string(),
                    buffer_offset: 224,
                },
                Uniform {
                    name: "gBilYJiku".to_string(),
                    buffer_offset: 272,
                },
                Uniform {
                    name: "gCDep".to_string(),
                    buffer_offset: 352,
                },
                Uniform {
                    name: "gDitTMAAVal".to_string(),
                    buffer_offset: 480,
                },
                Uniform {
                    name: "gDitVal".to_string(),
                    buffer_offset: 368,
                },
                Uniform {
                    name: "gEtcParm".to_string(),
                    buffer_offset: 320,
                },
                Uniform {
                    name: "gJitter".to_string(),
                    buffer_offset: 464,
                },
                Uniform {
                    name: "gLightShaft".to_string(),
                    buffer_offset: 624,
                },
                Uniform {
                    name: "gPreMat".to_string(),
                    buffer_offset: 384,
                },
                Uniform {
                    name: "gScreenSize".to_string(),
                    buffer_offset: 448,
                },
                Uniform {
                    name: "gViewYVec".to_string(),
                    buffer_offset: 336,
                },
                Uniform {
                    name: "gWetParam[0]".to_string(),
                    buffer_offset: 640,
                },
                Uniform {
                    name: "gmDiffPreMat".to_string(),
                    buffer_offset: 560,
                },
                Uniform {
                    name: "gmInvView".to_string(),
                    buffer_offset: 176,
                },
                Uniform {
                    name: "gmProj".to_string(),
                    buffer_offset: 48,
                },
                Uniform {
                    name: "gmProjNonJitter".to_string(),
                    buffer_offset: 496,
                },
                Uniform {
                    name: "gmView".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gmViewProj".to_string(),
                    buffer_offset: 112,
                },
                Uniform {
                    name: "gVolTexCalcWork[0]".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gmSkinMtx[0]".to_string(),
                    buffer_offset: 0,
                },
                Uniform {
                    name: "gmOldSkinMtx[0]".to_string(),
                    buffer_offset: 0,
                },
            ],
            samplers: Some(vec![
                Sampler {
                    name: "gTResidentTex04".to_string(),
                    unk1: 694,
                    handle: Handle {
                        handle: 6,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "gTResidentTex05".to_string(),
                    unk1: 696,
                    handle: Handle {
                        handle: 7,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "gTSpEffNoise1".to_string(),
                    unk1: 698,
                    handle: Handle {
                        handle: 5,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "gTSpEffVtxNoise1".to_string(),
                    unk1: 700,
                    handle: Handle {
                        handle: 3,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s0".to_string(),
                    unk1: 702,
                    handle: Handle {
                        handle: 0,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s1".to_string(),
                    unk1: 704,
                    handle: Handle {
                        handle: 1,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "s2".to_string(),
                    unk1: 706,
                    handle: Handle {
                        handle: 2,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
                Sampler {
                    name: "volTex0".to_string(),
                    unk1: 708,
                    handle: Handle {
                        handle: 4,
                        visibility: Visibility::Fragment,
                    },
                    unk: 0,
                },
            ]),
            ..Default::default()
        }
    }

    #[test]
    fn annotate_ch01011013_shd0056_vertex() {
        let glsl = indoc! {"
            layout (binding = 9, std140) uniform _vp_c8
            {
                precise vec4 data[4096];
            } vp_c8;
            
            layout (binding = 4, std140) uniform _vp_c3
            {
                precise vec4 data[4096];
            } vp_c3;
            
            layout (binding = 5, std140) uniform _vp_c4
            {
                precise vec4 data[4096];
            } vp_c4;
            
            layout (binding = 6, std140) uniform _vp_c5
            {
                precise vec4 data[4096];
            } vp_c5;
            
            layout (binding = 0, std430) buffer _vp_s0
            {
                uint data[];
            } vp_s0;
            
            layout (binding = 1, std430) buffer _vp_s1
            {
                uint data[];
            } vp_s1;
            
            layout (binding = 0) uniform sampler2D vp_t_tcb_E;
            layout (location = 0) in vec4 in_attr0;
            layout (location = 1) in vec4 in_attr1;
            layout (location = 2) in vec4 in_attr2;
            layout (location = 3) in vec4 in_attr3;
            layout (location = 4) in vec4 in_attr4;
            layout (location = 5) in vec4 in_attr5;
        "};

        let metadata = metadata();

        assert_eq!(
            indoc! {"
                layout(binding = 9, std140) uniform _U_CamoflageCalc {
                    precise vec4 data[4096];
                }U_CamoflageCalc;
                layout(binding = 4, std140) uniform _U_Static {
                    vec4 gmProj[4];
                    vec4 gmViewProj[4];
                    vec4 gmInvView[3];
                    vec4 gBilMat[3];
                    vec4 gBilYJiku[3];
                    vec4 gEtcParm;
                    vec4 gViewYVec;
                    vec4 gCDep;
                    vec4 gDitVal;
                    vec4 gPreMat[4];
                    vec4 gScreenSize;
                    vec4 gJitter;
                    vec4 gDitTMAAVal;
                    vec4 gmProjNonJitter[4];
                    vec4 gmDiffPreMat[4];
                    vec4 gLightShaft;
                    vec4 gWetParam;
                }U_Static;
                layout(binding = 5, std140) uniform _U_Mate {
                    vec4 gWrkFl4[3];
                    vec4 gWrkCol;
                }U_Mate;
                layout(binding = 6, std140) uniform _U_Mdl {
                    vec4 gmWorld[3];
                    vec4 gmWorldView[3];
                    vec4 gMdlParm;
                }U_Mdl;
                layout(binding = 0, std430) buffer _U_Bone {
                    uint data[];
                }U_Bone;
                layout(binding = 1, std430) buffer _U_OdB {
                    uint data[];
                }U_OdB;
                layout(binding = 0) uniform sampler2D vp_t_tcb_E;
                layout(location = 0) in vec4 vPos;
                layout(location = 1) in vec4 nWgtIdx;
                layout(location = 2) in vec4 vTex0;
                layout(location = 3) in vec4 vColor;
                layout(location = 4) in vec4 vNormal;
                layout(location = 5) in vec4 vTan;"
            },
            annotate_vertex(glsl.to_string(), &metadata)
        );
    }

    #[test]
    fn annotate_ch01011013_shd0056_fragment() {
        // Main function modified to test more indices.
        let glsl = indoc! {"
            layout (binding = 8, std140) uniform _fp_c7
            {
                precise vec4 data[4096];
            } fp_c7;

            layout (binding = 7, std140) uniform _fp_c6
            {
                precise vec4 data[4096];
            } fp_c6;

            layout (binding = 9, std140) uniform _fp_c8
            {
                precise vec4 data[4096];
            } fp_c8;

            layout (binding = 5, std140) uniform _fp_c4
            {
                precise vec4 data[4096];
            } fp_c4;

            layout (binding = 4, std140) uniform _fp_c3
            {
                precise vec4 data[4096];
            } fp_c3;

            layout (binding = 2, std140) uniform _fp_c1
            {
                precise vec4 data[4096];
            } fp_c1;

            layout (binding = 0) uniform sampler2D fp_t_tcb_C;
            layout (binding = 1) uniform sampler3D fp_t_tcb_10;
            layout (binding = 2) uniform sampler2D fp_t_tcb_A;
            layout (binding = 3) uniform sampler2D fp_t_tcb_16;
            layout (binding = 4) uniform sampler2D fp_t_tcb_14;
            layout (binding = 5) uniform sampler2D fp_t_tcb_8;
            layout (binding = 6) uniform sampler2D fp_t_tcb_12;

            void main() {
                out_attr0.x = fp_c4.data[2].x;
                out_attr0.y = fp_c4.data[3].y;
                out_attr0.z = fp_c4.data[4].z;
                out_attr0.w = temp_620;
                out_attr1.x = fp_c4.data[5].x;
                out_attr1.y = temp_623;
                out_attr1.z = 0.0;
                out_attr1.w = 0.00823529344;
            }
        "};

        let metadata = metadata();

        assert_eq!(
            indoc! {"
                layout(binding = 8, std140) uniform _U_RimBloomCalc {
                    precise vec4 data[4096];
                }U_RimBloomCalc;
                layout(binding = 7, std140) uniform _U_VolTexCalc {
                    precise vec4 data[4096];
                }U_VolTexCalc;
                layout(binding = 9, std140) uniform _U_CamoflageCalc {
                    precise vec4 data[4096];
                }U_CamoflageCalc;
                layout(binding = 5, std140) uniform _U_Mate {
                    vec4 gWrkFl4[3];
                    vec4 gWrkCol;
                }U_Mate;
                layout(binding = 4, std140) uniform _U_Static {
                    vec4 gmProj[4];
                    vec4 gmViewProj[4];
                    vec4 gmInvView[3];
                    vec4 gBilMat[3];
                    vec4 gBilYJiku[3];
                    vec4 gEtcParm;
                    vec4 gViewYVec;
                    vec4 gCDep;
                    vec4 gDitVal;
                    vec4 gPreMat[4];
                    vec4 gScreenSize;
                    vec4 gJitter;
                    vec4 gDitTMAAVal;
                    vec4 gmProjNonJitter[4];
                    vec4 gmDiffPreMat[4];
                    vec4 gLightShaft;
                    vec4 gWetParam;
                }U_Static;
                layout(binding = 2, std140) uniform _fp_c1 {
                    precise vec4 data[4096];
                }fp_c1;
                layout(binding = 0) uniform sampler2D s2;
                layout(binding = 1) uniform sampler3D volTex0;
                layout(binding = 2) uniform sampler2D s1;
                layout(binding = 3) uniform sampler2D gTResidentTex05;
                layout(binding = 4) uniform sampler2D gTResidentTex04;
                layout(binding = 5) uniform sampler2D s0;
                layout(binding = 6) uniform sampler2D gTSpEffNoise1;
                void main() {
                    out_attr0.x = U_Mate.gWrkFl4[0].x;
                    out_attr0.y = U_Mate.gWrkFl4[1].y;
                    out_attr0.z = U_Mate.gWrkFl4[2].z;
                    out_attr0.w = temp_620;
                    out_attr1.x = U_Mate.gWrkCol.x;
                    out_attr1.y = temp_623;
                    out_attr1.z = 0.;
                    out_attr1.w = 0.008235293;
                }
            "},
            annotate_fragment(glsl.to_string(), &metadata)
        );
    }
}
