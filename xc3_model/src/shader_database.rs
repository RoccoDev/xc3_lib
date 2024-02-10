//! Database for compiled shader metadata for more accurate rendering.
//!
//! In game shaders are precompiled and embedded in files like `.wismt`.
//! These types represent precomputed metadata like assignments to G-Buffer textures.
//! This is necessary for determining the usage of a texture like albedo or normal map
//! since the assignments are compiled into the shader code itself.
//!
//! Shader database JSON files should be generated using the xc3_shader CLI tool.
//! Applications can deserialize the JSON with [ShaderDatabase::from_file]
//! to avoid needing to generate this data at runtime.

use std::path::Path;

use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoadShaderDatabaseError {
    #[error("error writing files: {0}")]
    Io(#[from] std::io::Error),

    #[error("error serializing JSON file: {0}")]
    Json(#[from] serde_json::Error),
}

/// Metadata for the assigned [Shader] for all models and maps in a game dump.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShaderDatabase {
    /// The `.wimdo` file name without the extension and shader data for each file.
    pub files: IndexMap<String, Spch>,
    /// The `.wismhd` file name without the extension and shader data for each map.
    pub map_files: IndexMap<String, Map>,
}

impl ShaderDatabase {
    /// Loads and deserializes the JSON data from `path`.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, LoadShaderDatabaseError> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(Into::into)
    }
}

/// Shaders for the different map model types.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Map {
    pub map_models: Vec<Spch>,
    pub prop_models: Vec<Spch>,
    pub env_models: Vec<Spch>,
}

/// The decompiled shader data for a single shader container file.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Spch {
    pub programs: Vec<ShaderProgram>,
}

/// A collection of shaders.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShaderProgram {
    /// Some shaders have multiple NVSD sections, so the length may be greater than 1.
    pub shaders: Vec<Shader>,
}

// TODO: Document how to try sampler, constant, parameter in order.
/// The buffer elements, textures, and constants used to initialize each fragment output.
///
/// This assumes inputs are assigned directly to outputs without any modifications.
/// Fragment shaders typically only perform basic input and channel selection in practice.
///
/// This assignment information is needed to accurately recreate the G-Buffer texture values.
/// Renderers can generate unique shaders for each model
/// or select inputs in a shared shader at render time like xc3_wgpu.
/// Node based editors like Blender's shader editor should use these values
/// to determine how to construct node groups.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Shader {
    // TODO: make this a Vec<usize> and store a separate list of unique values to save space.
    /// A list of input dependencies like "s0.xyz" assigned to each output like "out_attr0.x".
    ///
    /// Each dependency can be thought of as a link
    /// between the dependency node and group output in a shader node graph.
    pub output_dependencies: IndexMap<String, Vec<Dependency>>,
}

/// A single buffer access like `UniformBuffer.field[0].y` in GLSL .
#[derive(Debug, PartialEq, Eq)]
pub struct BufferParameter {
    pub buffer: String,
    pub uniform: String,
    // TODO: make this a Vec<usize> and store a separate list of unique values to save space.
    pub output_dependencies: IndexMap<String, Vec<Dependency>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Dependency {
    Constant(OrderedFloat<f32>),
    Buffer(BufferDependency),
    Texture(TextureDependency),
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct BufferDependency {
    pub name: String,
    pub field: String,
    pub index: usize,
    pub channels: String,
}

/// A single texture access like `texture(s0, tex0.xy).rgb` in GLSL.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TextureDependency {
    pub name: String,
    pub channels: String,
    // TODO: Include the texture coordinate attribute name and UV offset/scale
    // TODO: This will require analyzing the vertex shader as well as the fragment shader.
}

impl Shader {
    /// Returns the sampler and channel index of the first material sampler assigned to the output
    /// or `None` if the output does not use a sampler.
    ///
    /// For example, an assignment of `"s3.y"` results in a sampler index of `3` and a channel index of `1`.
    pub fn sampler_channel_index(
        &self,
        output_index: usize,
        channel: char,
    ) -> Option<(usize, usize)> {
        let output = format!("o{output_index}.{channel}");

        // Find the first material referenced samplers like "s0" or "s1".
        let mut names_indices: Vec<_> = self
            .output_dependencies
            .get(&output)?
            .iter()
            .filter_map(|d| match d {
                Dependency::Texture(t) => Some((material_sampler_index(&t.name)?, &t.channels)),
                _ => None,
            })
            .collect();

        // TODO: Is there a better heuristic than always picking the lowest sampler index?
        names_indices.sort();
        let (sampler_index, channels) = names_indices.first()?;

        // Textures may have multiple accessed channels like normal maps.
        // First check if the current channel is used.
        // TODO: Does this always work as intended?
        let c = if channels.contains(channel) {
            channel
        } else {
            channels.chars().next().unwrap()
        };
        let channel_index = "xyzw".find(c).unwrap();
        Some((*sampler_index, channel_index))
    }

    /// Returns the float constant assigned directly to the output
    /// or `None` if the output does not use a constant.
    pub fn float_constant(&self, output_index: usize, channel: char) -> Option<f32> {
        let output = format!("o{output_index}.{channel}");

        // If a constant is assigned, it will be the only dependency.
        match self.output_dependencies.get(&output)?.first()? {
            Dependency::Constant(f) => Some(f.0),
            _ => None,
        }
    }

    /// Returns the uniform buffer parameter assigned directly to the output
    /// or `None` if the output does not use a parameter.
    pub fn buffer_parameter(
        &self,
        output_index: usize,
        channel: char,
    ) -> Option<&BufferDependency> {
        let output = format!("o{output_index}.{channel}");

        // If a parameter is assigned, it will be the only dependency.
        match self.output_dependencies.get(&output)?.first()? {
            Dependency::Buffer(b) => Some(b),
            _ => None,
        }
    }
}

fn material_sampler_index(sampler: &str) -> Option<usize> {
    // TODO: Just parse int?
    match sampler {
        "s0" => Some(0),
        "s1" => Some(1),
        "s2" => Some(2),
        "s3" => Some(3),
        "s4" => Some(4),
        "s5" => Some(5),
        "s6" => Some(6),
        "s7" => Some(7),
        "s8" => Some(8),
        "s9" => Some(9),
        // TODO: How to handle this case?
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_channel_assignment_empty() {
        let shader = Shader {
            output_dependencies: IndexMap::new(),
        };
        assert_eq!(None, shader.sampler_channel_index(0, 'x'));
    }

    #[test]
    fn material_channel_assignment_single_output_no_assignment() {
        let shader = Shader {
            output_dependencies: [("o0.x".to_string(), Vec::new())].into(),
        };
        assert_eq!(None, shader.sampler_channel_index(0, 'x'));
    }

    #[test]
    fn material_channel_assignment_multiple_output_assignment() {
        let shader = Shader {
            output_dependencies: [
                (
                    "o0.x".to_string(),
                    vec![Dependency::Texture(TextureDependency {
                        name: "s0".to_string(),
                        channels: "y".to_string(),
                    })],
                ),
                (
                    "o0.y".to_string(),
                    vec![
                        Dependency::Texture(TextureDependency {
                            name: "tex".to_string(),
                            channels: "xyz".to_string(),
                        }),
                        Dependency::Texture(TextureDependency {
                            name: "s2".to_string(),
                            channels: "z".to_string(),
                        }),
                    ],
                ),
                (
                    "o1.x".to_string(),
                    vec![Dependency::Texture(TextureDependency {
                        name: "s3".to_string(),
                        channels: "xyz".to_string(),
                    })],
                ),
            ]
            .into(),
        };
        assert_eq!(Some((2, 2)), shader.sampler_channel_index(0, 'y'));
    }

    #[test]
    fn float_constant_multiple_assigments() {
        let shader = Shader {
            output_dependencies: [
                (
                    "o0.x".to_string(),
                    vec![Dependency::Texture(TextureDependency {
                        name: "s0".to_string(),
                        channels: "y".to_string(),
                    })],
                ),
                (
                    "o0.y".to_string(),
                    vec![
                        Dependency::Texture(TextureDependency {
                            name: "tex".to_string(),
                            channels: "xyz".to_string(),
                        }),
                        Dependency::Texture(TextureDependency {
                            name: "s2".to_string(),
                            channels: "z".to_string(),
                        }),
                    ],
                ),
                ("o1.z".to_string(), vec![Dependency::Constant(0.5.into())]),
            ]
            .into(),
        };
        assert_eq!(None, shader.float_constant(0, 'x'));
        assert_eq!(Some(0.5), shader.float_constant(1, 'z'));
    }

    #[test]
    fn buffer_parameter_multiple_assigments() {
        let shader = Shader {
            output_dependencies: [
                (
                    "o0.x".to_string(),
                    vec![Dependency::Texture(TextureDependency {
                        name: "s0".to_string(),
                        channels: "y".to_string(),
                    })],
                ),
                (
                    "o0.y".to_string(),
                    vec![
                        Dependency::Texture(TextureDependency {
                            name: "tex".to_string(),
                            channels: "xyz".to_string(),
                        }),
                        Dependency::Texture(TextureDependency {
                            name: "s2".to_string(),
                            channels: "z".to_string(),
                        }),
                    ],
                ),
                (
                    "o1.z".to_string(),
                    vec![Dependency::Buffer(BufferDependency {
                        name: "U_Mate".to_string(),
                        field: "param".to_string(),
                        index: 31,
                        channels: "w".to_string(),
                    })],
                ),
            ]
            .into(),
        };
        assert_eq!(None, shader.buffer_parameter(0, 'x'));
        assert_eq!(
            Some(&BufferDependency {
                name: "U_Mate".to_string(),
                field: "param".to_string(),
                index: 31,
                channels: "w".to_string()
            }),
            shader.buffer_parameter(1, 'z')
        );
    }
}
