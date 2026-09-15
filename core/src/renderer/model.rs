use std::path::Path;
use std::sync::Arc;

use crate::renderer::extras::ModelVertex;
use crate::renderer::types::{BufferInitDescriptor, BufferUsage};
use crate::renderer::{Renderer, Texture};
use crate::transform::Transformable;
use crate::types::Color;

/// Per-model uniform data: the model matrix plus an optional highlight overlay (see
/// [`Model::highlight`]), laid out to match the `ModelTransform` struct declared in
/// `model_pipeline_vs.wgsl`/`_fs.wgsl` and `model_outline_vs.wgsl`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModelUniform {
    pub(crate) model: [[f32; 4]; 4],
    pub(crate) highlight: [f32; 4],
}

#[derive(Clone)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    /// Model matrix applied to every vertex in world space. Defaults to
    /// [`glam::Mat4::IDENTITY`]. Prefer building it up through the [`Transformable`] methods.
    pub transform: glam::Mat4,
    /// RGBA overlay blended over the model's shaded color
    /// (`mix(shaded, highlight.rgb, highlight.a)` in the fragment shader). Alpha 0 (the default)
    /// means no highlight at all. Intended for editor-style hover/selection feedback rather than
    /// gameplay use.
    pub highlight: Color,
    /// Center of the model's bounding sphere, in local (pre-transform) space.
    pub bounding_center: glam::Vec3,
    /// Radius of the model's bounding sphere, in local (pre-transform) space. Computed once at
    /// load time since the raw vertex positions aren't retained after upload to the GPU.
    pub bounding_radius: f32,
    pub(crate) transform_buffer: wgpu::Buffer,
    pub(crate) transform_bind_group: wgpu::BindGroup,
}

#[derive(Clone)]
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub material: usize,
    /// Local-space vertex positions, retained after upload to the GPU so callers (e.g. the
    /// editor's ray/triangle picking) can test against the actual mesh geometry rather than just
    /// its bounding sphere.
    pub positions: Vec<glam::Vec3>,
    /// Triangle-list indices into `positions`, retained alongside it for the same reason.
    pub indices: Vec<u32>,
}

#[derive(Clone)]
pub struct Material {
    pub name: String,
    pub diffuse_texture: Arc<Texture>,
}

impl Model {
    /// Loads a Wavefront OBJ model (and its referenced `.mtl` materials and textures) from
    /// `path`. Texture paths in the `.mtl` file are resolved relative to the OBJ's directory.
    pub fn load(renderer: &Renderer, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = crate::util::resolve_resource_path(path);
        let parent = path.parent().unwrap_or_else(|| Path::new(""));

        let (obj_models, obj_materials) = tobj::load_obj(
            &path,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )?;
        let obj_materials = obj_materials?;

        let mut materials: Vec<Material> = obj_materials
            .into_iter()
            .map(|mat| {
                let diffuse_texture = match &mat.diffuse_texture {
                    Some(texture_file) => {
                        Texture::from_path(renderer, parent.join(texture_file)).map_err(|e| anyhow::anyhow!(e))?
                    }
                    None => Texture::from_color(renderer, [255, 255, 255, 255]).map_err(|e| anyhow::anyhow!(e))?,
                };
                Ok(Material {
                    name: mat.name,
                    diffuse_texture: Arc::new(diffuse_texture),
                })
            })
            .collect::<anyhow::Result<_>>()?;

        if materials.is_empty() {
            let diffuse_texture = Texture::from_color(renderer, [255, 255, 255, 255]).map_err(|e| anyhow::anyhow!(e))?;
            materials.push(Material {
                name: "default".to_string(),
                diffuse_texture: Arc::new(diffuse_texture),
            });
        }
        let fallback_material = materials.len() - 1;

        let mut bounds_min = glam::Vec3::splat(f32::INFINITY);
        let mut bounds_max = glam::Vec3::splat(f32::NEG_INFINITY);

        let meshes = obj_models
            .into_iter()
            .map(|obj_model| {
                let mesh = obj_model.mesh;
                let vertex_count = mesh.positions.len() / 3;
                let vertices: Vec<ModelVertex> = (0..vertex_count)
                    .map(|i| {
                        let position = [mesh.positions[i * 3], mesh.positions[i * 3 + 1], mesh.positions[i * 3 + 2]];
                        let p = glam::Vec3::from_array(position);
                        bounds_min = bounds_min.min(p);
                        bounds_max = bounds_max.max(p);
                        ModelVertex {
                            position,
                            tex_coords: if mesh.texcoords.is_empty() {
                                [0.0, 0.0]
                            } else {
                                // OBJ texture coordinates are Y-up; wgpu textures are Y-down.
                                [mesh.texcoords[i * 2], 1.0 - mesh.texcoords[i * 2 + 1]]
                            },
                            normal: if mesh.normals.is_empty() {
                                [0.0, 0.0, 0.0]
                            } else {
                                [mesh.normals[i * 3], mesh.normals[i * 3 + 1], mesh.normals[i * 3 + 2]]
                            },
                        }
                    })
                    .collect();

                let vertex_buffer = renderer.create_buffer_init(&BufferInitDescriptor {
                    label: Some(&format!("{} Vertex Buffer", obj_model.name)),
                    contents: crate::util::slice_to_bytes(&vertices),
                    usage: BufferUsage::Vertex,
                });
                let index_buffer = renderer.create_buffer_init(&BufferInitDescriptor {
                    label: Some(&format!("{} Index Buffer", obj_model.name)),
                    contents: crate::util::slice_to_bytes(&mesh.indices),
                    usage: BufferUsage::Index,
                });

                let positions: Vec<glam::Vec3> = vertices.iter().map(|v| glam::Vec3::from_array(v.position)).collect();

                Mesh {
                    name: obj_model.name,
                    vertex_buffer,
                    index_buffer,
                    num_indices: mesh.indices.len() as u32,
                    material: mesh.material_id.unwrap_or(fallback_material).min(fallback_material),
                    positions,
                    indices: mesh.indices,
                }
            })
            .collect();

        let (bounding_center, bounding_radius) = if bounds_min.x.is_finite() {
            ((bounds_min + bounds_max) * 0.5, (bounds_max - bounds_min).length() * 0.5)
        } else {
            (glam::Vec3::ZERO, 0.0)
        };

        let transform_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Model Transform Buffer"),
            size: std::mem::size_of::<ModelUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        renderer.queue.write_buffer(
            &transform_buffer,
            0,
            crate::util::slice_to_bytes(&[ModelUniform {
                model: glam::Mat4::IDENTITY.to_cols_array_2d(),
                highlight: [1.0, 1.0, 1.0, 0.0],
            }]),
        );
        let transform_bind_group = renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &renderer.model_transform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: transform_buffer.as_entire_binding(),
            }],
            label: Some("model_transform_bind_group"),
        });

        Ok(Self {
            meshes,
            materials,
            transform: glam::Mat4::IDENTITY,
            highlight: Color::new(1.0, 1.0, 1.0, 0.0),
            bounding_center,
            bounding_radius,
            transform_buffer,
            transform_bind_group,
        })
    }
}

impl Transformable for Model {
    fn transform(&self) -> glam::Mat4 {
        self.transform
    }

    fn transform_mut(&mut self) -> &mut glam::Mat4 {
        &mut self.transform
    }
}
