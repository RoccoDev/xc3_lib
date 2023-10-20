// File automatically generated by build.rs.
// Changes made to this file will not be saved.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera {
    pub view: glam::Mat4,
    pub view_projection: glam::Mat4,
    pub position: glam::Vec4,
}
const _: () = assert!(
    std::mem::size_of:: < Camera > () == 144, "size of Camera does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(Camera, view) == 0, "offset of Camera.view does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(Camera, view_projection) == 64,
    "offset of Camera.view_projection does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(Camera, position) == 128,
    "offset of Camera.position does not match WGSL"
);
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PerGroup {
    pub enable_skinning: glam::UVec4,
    pub animated_transforms: [glam::Mat4; 256],
}
const _: () = assert!(
    std::mem::size_of:: < PerGroup > () == 16400, "size of PerGroup does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(PerGroup, enable_skinning) == 0,
    "offset of PerGroup.enable_skinning does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(PerGroup, animated_transforms) == 16,
    "offset of PerGroup.animated_transforms does not match WGSL"
);
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GBufferAssignment {
    pub sampler_indices: glam::IVec4,
    pub channel_indices: glam::UVec4,
}
const _: () = assert!(
    std::mem::size_of:: < GBufferAssignment > () == 32,
    "size of GBufferAssignment does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(GBufferAssignment, sampler_indices) == 0,
    "offset of GBufferAssignment.sampler_indices does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(GBufferAssignment, channel_indices) == 16,
    "offset of GBufferAssignment.channel_indices does not match WGSL"
);
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PerMaterial {
    pub mat_color: glam::Vec4,
    pub gbuffer_assignments: [GBufferAssignment; 6],
    pub gbuffer_defaults: [glam::Vec4; 6],
    pub alpha_test_texture: glam::IVec4,
    pub alpha_test_ref: glam::Vec4,
}
const _: () = assert!(
    std::mem::size_of:: < PerMaterial > () == 336,
    "size of PerMaterial does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(PerMaterial, mat_color) == 0,
    "offset of PerMaterial.mat_color does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(PerMaterial, gbuffer_assignments) == 16,
    "offset of PerMaterial.gbuffer_assignments does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(PerMaterial, gbuffer_defaults) == 208,
    "offset of PerMaterial.gbuffer_defaults does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(PerMaterial, alpha_test_texture) == 304,
    "offset of PerMaterial.alpha_test_texture does not match WGSL"
);
const _: () = assert!(
    memoffset::offset_of!(PerMaterial, alpha_test_ref) == 320,
    "offset of PerMaterial.alpha_test_ref does not match WGSL"
);
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexInput {
    pub position: glam::Vec3,
    pub weight_index: u32,
    pub vertex_color: glam::Vec4,
    pub normal: glam::Vec4,
    pub tangent: glam::Vec4,
    pub uv1: glam::Vec4,
}
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceInput {
    pub model_matrix_0: glam::Vec4,
    pub model_matrix_1: glam::Vec4,
    pub model_matrix_2: glam::Vec4,
    pub model_matrix_3: glam::Vec4,
}
pub mod bind_groups {
    pub struct BindGroup0(wgpu::BindGroup);
    pub struct BindGroupLayout0<'a> {
        pub camera: wgpu::BufferBinding<'a>,
    }
    const LAYOUT_DESCRIPTOR0: wgpu::BindGroupLayoutDescriptor = wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    };
    impl BindGroup0 {
        pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
            device.create_bind_group_layout(&LAYOUT_DESCRIPTOR0)
        }
        pub fn from_bindings(device: &wgpu::Device, bindings: BindGroupLayout0) -> Self {
            let bind_group_layout = device.create_bind_group_layout(&LAYOUT_DESCRIPTOR0);
            let bind_group = device
                .create_bind_group(
                    &wgpu::BindGroupDescriptor {
                        layout: &bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::Buffer(bindings.camera),
                            },
                        ],
                        label: None,
                    },
                );
            Self(bind_group)
        }
        pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
            render_pass.set_bind_group(0, &self.0, &[]);
        }
    }
    pub struct BindGroup1(wgpu::BindGroup);
    pub struct BindGroupLayout1<'a> {
        pub per_group: wgpu::BufferBinding<'a>,
    }
    const LAYOUT_DESCRIPTOR1: wgpu::BindGroupLayoutDescriptor = wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    };
    impl BindGroup1 {
        pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
            device.create_bind_group_layout(&LAYOUT_DESCRIPTOR1)
        }
        pub fn from_bindings(device: &wgpu::Device, bindings: BindGroupLayout1) -> Self {
            let bind_group_layout = device.create_bind_group_layout(&LAYOUT_DESCRIPTOR1);
            let bind_group = device
                .create_bind_group(
                    &wgpu::BindGroupDescriptor {
                        layout: &bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::Buffer(bindings.per_group),
                            },
                        ],
                        label: None,
                    },
                );
            Self(bind_group)
        }
        pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
            render_pass.set_bind_group(1, &self.0, &[]);
        }
    }
    pub struct BindGroup2(wgpu::BindGroup);
    pub struct BindGroupLayout2<'a> {
        pub s0: &'a wgpu::TextureView,
        pub s1: &'a wgpu::TextureView,
        pub s2: &'a wgpu::TextureView,
        pub s3: &'a wgpu::TextureView,
        pub s4: &'a wgpu::TextureView,
        pub s5: &'a wgpu::TextureView,
        pub s6: &'a wgpu::TextureView,
        pub s7: &'a wgpu::TextureView,
        pub s8: &'a wgpu::TextureView,
        pub s9: &'a wgpu::TextureView,
        pub s0_sampler: &'a wgpu::Sampler,
        pub s1_sampler: &'a wgpu::Sampler,
        pub s2_sampler: &'a wgpu::Sampler,
        pub s3_sampler: &'a wgpu::Sampler,
        pub s4_sampler: &'a wgpu::Sampler,
        pub s5_sampler: &'a wgpu::Sampler,
        pub s6_sampler: &'a wgpu::Sampler,
        pub s7_sampler: &'a wgpu::Sampler,
        pub s8_sampler: &'a wgpu::Sampler,
        pub s9_sampler: &'a wgpu::Sampler,
        pub per_material: wgpu::BufferBinding<'a>,
    }
    const LAYOUT_DESCRIPTOR2: wgpu::BindGroupLayoutDescriptor = wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true,
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true,
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true,
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true,
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true,
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true,
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 6,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true,
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 7,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true,
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 8,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true,
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 9,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true,
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 10,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 11,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 12,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 13,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 14,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 15,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 16,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 17,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 18,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 19,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 20,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    };
    impl BindGroup2 {
        pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
            device.create_bind_group_layout(&LAYOUT_DESCRIPTOR2)
        }
        pub fn from_bindings(device: &wgpu::Device, bindings: BindGroupLayout2) -> Self {
            let bind_group_layout = device.create_bind_group_layout(&LAYOUT_DESCRIPTOR2);
            let bind_group = device
                .create_bind_group(
                    &wgpu::BindGroupDescriptor {
                        layout: &bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(bindings.s0),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(bindings.s1),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(bindings.s2),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::TextureView(bindings.s3),
                            },
                            wgpu::BindGroupEntry {
                                binding: 4,
                                resource: wgpu::BindingResource::TextureView(bindings.s4),
                            },
                            wgpu::BindGroupEntry {
                                binding: 5,
                                resource: wgpu::BindingResource::TextureView(bindings.s5),
                            },
                            wgpu::BindGroupEntry {
                                binding: 6,
                                resource: wgpu::BindingResource::TextureView(bindings.s6),
                            },
                            wgpu::BindGroupEntry {
                                binding: 7,
                                resource: wgpu::BindingResource::TextureView(bindings.s7),
                            },
                            wgpu::BindGroupEntry {
                                binding: 8,
                                resource: wgpu::BindingResource::TextureView(bindings.s8),
                            },
                            wgpu::BindGroupEntry {
                                binding: 9,
                                resource: wgpu::BindingResource::TextureView(bindings.s9),
                            },
                            wgpu::BindGroupEntry {
                                binding: 10,
                                resource: wgpu::BindingResource::Sampler(
                                    bindings.s0_sampler,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 11,
                                resource: wgpu::BindingResource::Sampler(
                                    bindings.s1_sampler,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 12,
                                resource: wgpu::BindingResource::Sampler(
                                    bindings.s2_sampler,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 13,
                                resource: wgpu::BindingResource::Sampler(
                                    bindings.s3_sampler,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 14,
                                resource: wgpu::BindingResource::Sampler(
                                    bindings.s4_sampler,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 15,
                                resource: wgpu::BindingResource::Sampler(
                                    bindings.s5_sampler,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 16,
                                resource: wgpu::BindingResource::Sampler(
                                    bindings.s6_sampler,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 17,
                                resource: wgpu::BindingResource::Sampler(
                                    bindings.s7_sampler,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 18,
                                resource: wgpu::BindingResource::Sampler(
                                    bindings.s8_sampler,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 19,
                                resource: wgpu::BindingResource::Sampler(
                                    bindings.s9_sampler,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 20,
                                resource: wgpu::BindingResource::Buffer(
                                    bindings.per_material,
                                ),
                            },
                        ],
                        label: None,
                    },
                );
            Self(bind_group)
        }
        pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
            render_pass.set_bind_group(2, &self.0, &[]);
        }
    }
    pub struct BindGroup3(wgpu::BindGroup);
    pub struct BindGroupLayout3<'a> {
        pub bone_indices: wgpu::BufferBinding<'a>,
        pub skin_weights: wgpu::BufferBinding<'a>,
    }
    const LAYOUT_DESCRIPTOR3: wgpu::BindGroupLayoutDescriptor = wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {
                        read_only: true,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {
                        read_only: true,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    };
    impl BindGroup3 {
        pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
            device.create_bind_group_layout(&LAYOUT_DESCRIPTOR3)
        }
        pub fn from_bindings(device: &wgpu::Device, bindings: BindGroupLayout3) -> Self {
            let bind_group_layout = device.create_bind_group_layout(&LAYOUT_DESCRIPTOR3);
            let bind_group = device
                .create_bind_group(
                    &wgpu::BindGroupDescriptor {
                        layout: &bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::Buffer(
                                    bindings.bone_indices,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Buffer(
                                    bindings.skin_weights,
                                ),
                            },
                        ],
                        label: None,
                    },
                );
            Self(bind_group)
        }
        pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
            render_pass.set_bind_group(3, &self.0, &[]);
        }
    }
    pub struct BindGroups<'a> {
        pub bind_group0: &'a BindGroup0,
        pub bind_group1: &'a BindGroup1,
        pub bind_group2: &'a BindGroup2,
        pub bind_group3: &'a BindGroup3,
    }
    pub fn set_bind_groups<'a>(
        pass: &mut wgpu::RenderPass<'a>,
        bind_groups: BindGroups<'a>,
    ) {
        bind_groups.bind_group0.set(pass);
        bind_groups.bind_group1.set(pass);
        bind_groups.bind_group2.set(pass);
        bind_groups.bind_group3.set(pass);
    }
}
pub mod vertex {
    impl super::VertexInput {
        pub const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: memoffset::offset_of!(super::VertexInput, position) as u64,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32,
                offset: memoffset::offset_of!(super::VertexInput, weight_index) as u64,
                shader_location: 2,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: memoffset::offset_of!(super::VertexInput, vertex_color) as u64,
                shader_location: 3,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: memoffset::offset_of!(super::VertexInput, normal) as u64,
                shader_location: 4,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: memoffset::offset_of!(super::VertexInput, tangent) as u64,
                shader_location: 5,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: memoffset::offset_of!(super::VertexInput, uv1) as u64,
                shader_location: 6,
            },
        ];
        pub const fn vertex_buffer_layout(
            step_mode: wgpu::VertexStepMode,
        ) -> wgpu::VertexBufferLayout<'static> {
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<super::VertexInput>() as u64,
                step_mode,
                attributes: &super::VertexInput::VERTEX_ATTRIBUTES,
            }
        }
    }
    impl super::InstanceInput {
        pub const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: memoffset::offset_of!(super::InstanceInput, model_matrix_0)
                    as u64,
                shader_location: 7,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: memoffset::offset_of!(super::InstanceInput, model_matrix_1)
                    as u64,
                shader_location: 8,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: memoffset::offset_of!(super::InstanceInput, model_matrix_2)
                    as u64,
                shader_location: 9,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: memoffset::offset_of!(super::InstanceInput, model_matrix_3)
                    as u64,
                shader_location: 10,
            },
        ];
        pub const fn vertex_buffer_layout(
            step_mode: wgpu::VertexStepMode,
        ) -> wgpu::VertexBufferLayout<'static> {
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<super::InstanceInput>() as u64,
                step_mode,
                attributes: &super::InstanceInput::VERTEX_ATTRIBUTES,
            }
        }
    }
}
pub const ENTRY_VS_MAIN: &str = "vs_main";
pub const ENTRY_FS_MAIN: &str = "fs_main";
pub struct VertexEntry<const N: usize> {
    entry_point: &'static str,
    buffers: [wgpu::VertexBufferLayout<'static>; N],
}
pub fn vertex_state<'a, const N: usize>(
    module: &'a wgpu::ShaderModule,
    entry: &'a VertexEntry<N>,
) -> wgpu::VertexState<'a> {
    wgpu::VertexState {
        module,
        entry_point: entry.entry_point,
        buffers: &entry.buffers,
    }
}
pub fn vs_main_entry(
    vertex_input: wgpu::VertexStepMode,
    instance_input: wgpu::VertexStepMode,
) -> VertexEntry<2> {
    VertexEntry {
        entry_point: ENTRY_VS_MAIN,
        buffers: [
            VertexInput::vertex_buffer_layout(vertex_input),
            InstanceInput::vertex_buffer_layout(instance_input),
        ],
    }
}
pub fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let source = std::borrow::Cow::Borrowed(include_str!("model.wgsl"));
    device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(source),
        })
}
pub fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    device
        .create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    &bind_groups::BindGroup0::get_bind_group_layout(device),
                    &bind_groups::BindGroup1::get_bind_group_layout(device),
                    &bind_groups::BindGroup2::get_bind_group_layout(device),
                    &bind_groups::BindGroup3::get_bind_group_layout(device),
                ],
                push_constant_ranges: &[],
            },
        )
}
