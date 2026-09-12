use std::path::Path;
use std::sync::Arc;

use crate::renderer::extras::ModelVertex;
use crate::renderer::types::{BufferInitDescriptor, BufferUsage};
use crate::renderer::{Renderer, Texture};
use crate::transform::Transformable;

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    /// Model matrix applied to every vertex in world space. Defaults to
    /// [`glam::Mat4::IDENTITY`]. Prefer building it up through the [`Transformable`] methods.
    pub transform: glam::Mat4,
    pub(crate) transform_buffer: wgpu::Buffer,
    pub(crate) transform_bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub material: usize,
}

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

        let meshes = obj_models
            .into_iter()
            .map(|obj_model| {
                let mesh = obj_model.mesh;
                let vertex_count = mesh.positions.len() / 3;
                let vertices: Vec<ModelVertex> = (0..vertex_count)
                    .map(|i| ModelVertex {
                        position: [mesh.positions[i * 3], mesh.positions[i * 3 + 1], mesh.positions[i * 3 + 2]],
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

                Mesh {
                    name: obj_model.name,
                    vertex_buffer,
                    index_buffer,
                    num_indices: mesh.indices.len() as u32,
                    material: mesh.material_id.unwrap_or(fallback_material).min(fallback_material),
                }
            })
            .collect();

        let transform_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Model Transform Buffer"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        renderer.queue.write_buffer(
            &transform_buffer,
            0,
            crate::util::slice_to_bytes(&[glam::Mat4::IDENTITY.to_cols_array_2d()]),
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
