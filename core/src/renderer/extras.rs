use crate::renderer::DrawPass;
use crate::types::Color;

const VS_SRC: &str = include_str!("../../shaders/shape_pipeline_vs.wgsl");
const FS_SRC: &str = include_str!("../../shaders/shape_pipeline_fs.wgsl");

const SPRITE_VS_SRC: &str = include_str!("../../shaders/sprite_pipeline_vs.wgsl");
const SPRITE_FS_SRC: &str = include_str!("../../shaders/sprite_pipeline_fs.wgsl");

const WORLD_SPRITE_VS_SRC: &str = include_str!("../../shaders/world_sprite_pipeline_vs.wgsl");

/// Format of the renderer's depth buffer. Every pipeline drawn in the main pass must declare a
/// depth-stencil state using this format.
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Depth state for screen-space drawing: never tested, never written, so UI paints in call
/// order on top of whatever the world left behind.
pub(crate) fn overlay_depth_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(false),
        depth_compare: Some(wgpu::CompareFunction::Always),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

/// Depth state for world-space drawing: nearer fragments win and update the buffer.
pub(crate) fn world_depth_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Less),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

#[repr(C)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
struct SpriteVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
struct WorldSpriteVertex {
    pos: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
}

pub fn create_shape_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shape Shader VS"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(VS_SRC)),
    });
    let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shape Shader FS"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(FS_SRC)),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Shape Pipeline Layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    let wgpu_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Shape Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: Some("vs_main"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: std::mem::size_of::<f32>() as u64 * 2,
                        shader_location: 1,
                    },
                ],
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(overlay_depth_state()),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    });
    wgpu_pipeline
}

pub fn create_sprite_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat, texture_layout: &wgpu::BindGroupLayout) -> wgpu::RenderPipeline {
    let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Sprite Shader VS"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SPRITE_VS_SRC)),
    });
    let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Sprite Shader FS"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SPRITE_FS_SRC)),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Sprite Pipeline Layout"),
        bind_group_layouts: &[Some(texture_layout)],
        immediate_size: 0,
    });

    let wgpu_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Sprite Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: Some("vs_main"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SpriteVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: std::mem::size_of::<f32>() as u64 * 2,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: std::mem::size_of::<f32>() as u64 * 4,
                        shader_location: 2,
                    },
                ],
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(overlay_depth_state()),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    });
    wgpu_pipeline
}

pub fn create_world_sprite_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    texture_layout: &wgpu::BindGroupLayout,
    camera_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("World Sprite Shader VS"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(WORLD_SPRITE_VS_SRC)),
    });
    let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("World Sprite Shader FS"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SPRITE_FS_SRC)),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("World Sprite Pipeline Layout"),
        bind_group_layouts: &[Some(texture_layout), Some(camera_layout)],
        immediate_size: 0,
    });

    let wgpu_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("World Sprite Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: Some("vs_main"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<WorldSpriteVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: std::mem::size_of::<f32>() as u64 * 3,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: std::mem::size_of::<f32>() as u64 * 5,
                        shader_location: 2,
                    },
                ],
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(world_depth_state()),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    });
    wgpu_pipeline
}

impl DrawPass<'_> {
    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Color) {
        let c = color.as_wgpu_color();
        let rgba = [c.r as f32, c.g as f32, c.b as f32, c.a as f32];
        let mut verts: Vec<Vertex> = Vec::new();

        if thickness <= 0.0 {
            let x2 = x + w;
            let y2 = y + h;
            let p0 = self.to_clip(x, y);
            let p1 = self.to_clip(x2, y);
            let p2 = self.to_clip(x, y2);
            let p3 = self.to_clip(x2, y2);

            verts.push(Vertex { pos: p0, color: rgba });
            verts.push(Vertex { pos: p1, color: rgba });
            verts.push(Vertex { pos: p2, color: rgba });
            verts.push(Vertex { pos: p1, color: rgba });
            verts.push(Vertex { pos: p3, color: rgba });
            verts.push(Vertex { pos: p2, color: rgba });
        } else {
            let th = thickness;
            for &(px, py) in &[
                (x, y), (x + w, y), (x, y + th),
                (x + w, y), (x + w, y + th), (x, y + th),
            ] {
                verts.push(Vertex { pos: self.to_clip(px, py), color: rgba });
            }
            for &(px, py) in &[
                (x, y + h - th), (x + w, y + h - th), (x, y + h),
                (x + w, y + h - th), (x + w, y + h), (x, y + h),
            ] {
                verts.push(Vertex { pos: self.to_clip(px, py), color: rgba });
            }
            for &(px, py) in &[
                (x, y), (x + th, y), (x, y + h),
                (x + th, y), (x + th, y + h), (x, y + h),
            ] {
                verts.push(Vertex { pos: self.to_clip(px, py), color: rgba });
            }
            for &(px, py) in &[
                (x + w - th, y), (x + w, y), (x + w - th, y + h),
                (x + w, y), (x + w, y + h), (x + w - th, y + h),
            ] {
                verts.push(Vertex { pos: self.to_clip(px, py), color: rgba });
            }
        }

        self.upload_and_draw(&verts);
    }

    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        if len == 0.0 {
            return;
        }

        let c = color.as_wgpu_color();
        let rgba = [c.r as f32, c.g as f32, c.b as f32, c.a as f32];

        let hw = thickness * 0.5;
        let px = -dy / len * hw;
        let py = dx / len * hw;

        let p0 = self.to_clip(x1 + px, y1 + py);
        let p1 = self.to_clip(x1 - px, y1 - py);
        let p2 = self.to_clip(x2 + px, y2 + py);
        let p3 = self.to_clip(x2 - px, y2 - py);

        let verts = vec![
            Vertex { pos: p0, color: rgba },
            Vertex { pos: p1, color: rgba },
            Vertex { pos: p2, color: rgba },
            Vertex { pos: p1, color: rgba },
            Vertex { pos: p3, color: rgba },
            Vertex { pos: p2, color: rgba },
        ];

        self.upload_and_draw(&verts);
    }

    pub fn draw_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, segments: u32, thickness: f32, color: Color) {
        if segments < 3 {
            return;
        }

        let c = color.as_wgpu_color();
        let rgba = [c.r as f32, c.g as f32, c.b as f32, c.a as f32];

        let step = std::f32::consts::TAU / segments as f32;
        let mut verts = Vec::new();

        if thickness <= 0.0 {
            let center = self.to_clip(cx, cy);
            for i in 0..segments {
                let a1 = i as f32 * step;
                let a2 = (i + 1) as f32 * step;
                let p1 = self.to_clip(cx + rx * a1.cos(), cy + ry * a1.sin());
                let p2 = self.to_clip(cx + rx * a2.cos(), cy + ry * a2.sin());
                verts.push(Vertex { pos: center, color: rgba });
                verts.push(Vertex { pos: p1, color: rgba });
                verts.push(Vertex { pos: p2, color: rgba });
            }
        } else {
            let hw = thickness * 0.5;
            for i in 0..segments {
                let a1 = i as f32 * step;
                let a2 = (i + 1) as f32 * step;
                let cos1 = a1.cos();
                let sin1 = a1.sin();
                let cos2 = a2.cos();
                let sin2 = a2.sin();

                let p_o1 = self.to_clip(cx + (rx + hw) * cos1, cy + (ry + hw) * sin1);
                let p_i1 = self.to_clip(cx + (rx - hw) * cos1, cy + (ry - hw) * sin1);
                let p_o2 = self.to_clip(cx + (rx + hw) * cos2, cy + (ry + hw) * sin2);
                let p_i2 = self.to_clip(cx + (rx - hw) * cos2, cy + (ry - hw) * sin2);

                verts.push(Vertex { pos: p_o1, color: rgba });
                verts.push(Vertex { pos: p_i1, color: rgba });
                verts.push(Vertex { pos: p_o2, color: rgba });
                verts.push(Vertex { pos: p_i1, color: rgba });
                verts.push(Vertex { pos: p_i2, color: rgba });
                verts.push(Vertex { pos: p_o2, color: rgba });
            }
        }

        self.upload_and_draw(&verts);
    }

    fn to_clip(&self, x: f32, y: f32) -> [f32; 2] {
        let w = (self.screen_w.max(1)) as f32;
        let h = (self.screen_h.max(1)) as f32;
        [(x / w) * 2.0 - 1.0, -((y / h) * 2.0 - 1.0)]
    }

    fn upload_and_draw(&mut self, verts: &[Vertex]) {
        if verts.is_empty() {
            return;
        }
        let data = crate::util::slice_to_bytes(verts);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shape Vert Buffer"),
            size: data.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buffer, 0, data);
        self.pass.set_pipeline(&self.immediate_pipeline);
        self.pass.set_vertex_buffer(0, buffer.slice(..data.len() as u64));
        self.pass.draw(0..verts.len() as u32, 0..1);
    }

    /// Draws a sprite in screen space (pixel coordinates), ignoring the camera. Use for UI/HUD elements.
    ///
    /// `model` transforms the quad in the sprite's local space, where the origin is the
    /// top-left corner and the quad spans `(0, 0)..(w, h)`. The result is then offset by
    /// `(x, y)`. Only the XY components of the transform are used.
    pub fn draw_ui_sprite(
        &mut self,
        x: f32, y: f32, w: f32, h: f32,
        src_x: f32, src_y: f32, src_w: f32, src_h: f32, tex_w: f32, tex_h: f32,
        color: Color,
        model: glam::Mat4,
        bind_group: &wgpu::BindGroup,
    ) {
        let c = color.as_wgpu_color();
        let rgba = [c.r as f32, c.g as f32, c.b as f32, c.a as f32];

        let corner = |lx: f32, ly: f32| {
            let p = model.transform_point3(glam::Vec3::new(lx, ly, 0.0));
            self.to_clip(x + p.x, y + p.y)
        };

        let p0 = corner(0.0, 0.0);
        let p1 = corner(w, 0.0);
        let p2 = corner(0.0, h);
        let p3 = corner(w, h);

        let uv0 = [src_x / tex_w, src_y / tex_h];
        let uv1 = [(src_x + src_w) / tex_w, src_y / tex_h];
        let uv2 = [src_x / tex_w, (src_y + src_h) / tex_h];
        let uv3 = [(src_x + src_w) / tex_w, (src_y + src_h) / tex_h];

        let verts = vec![
            SpriteVertex { pos: p0, uv: uv0, color: rgba },
            SpriteVertex { pos: p1, uv: uv1, color: rgba },
            SpriteVertex { pos: p2, uv: uv2, color: rgba },
            SpriteVertex { pos: p1, uv: uv1, color: rgba },
            SpriteVertex { pos: p3, uv: uv3, color: rgba },
            SpriteVertex { pos: p2, uv: uv2, color: rgba },
        ];

        let data = crate::util::slice_to_bytes(&verts);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Vert Buffer"),
            size: data.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buffer, 0, data);
        self.pass.set_pipeline(&self.sprite_pipeline);
        self.pass.set_bind_group(0, bind_group, &[]);
        self.pass.set_vertex_buffer(0, buffer.slice(..data.len() as u64));
        self.pass.draw(0..verts.len() as u32, 0..1);
    }

    /// Draws a sprite as a quad in world space (on the XY plane at depth `z`), transformed by the camera's view-projection matrix.
    ///
    /// `model` transforms the quad in the sprite's local space, where the origin is the
    /// bottom-left corner and the quad spans `(0, 0)..(w, h)`. The result is then offset by
    /// `(x, y, z)`, so a rotation about Z spins the quad in place rather than orbiting the
    /// world origin.
    pub fn draw_world_sprite(
        &mut self,
        x: f32, y: f32, z: f32, w: f32, h: f32,
        src_x: f32, src_y: f32, src_w: f32, src_h: f32, tex_w: f32, tex_h: f32,
        color: Color,
        model: glam::Mat4,
        bind_group: &wgpu::BindGroup,
    ) {
        let c = color.as_wgpu_color();
        let rgba = [c.r as f32, c.g as f32, c.b as f32, c.a as f32];

        let origin = glam::Vec3::new(x, y, z);
        let corner = |lx: f32, ly: f32| {
            let p = origin + model.transform_point3(glam::Vec3::new(lx, ly, 0.0));
            [p.x, p.y, p.z]
        };

        let p0 = corner(0.0, 0.0);
        let p1 = corner(w, 0.0);
        let p2 = corner(0.0, h);
        let p3 = corner(w, h);

        // World space is Y-up but texture space is Y-down, so the quad's top
        // edge (y + h) samples the top of the source rect, not the bottom.
        let u_left = src_x / tex_w;
        let u_right = (src_x + src_w) / tex_w;
        let v_top = src_y / tex_h;
        let v_bottom = (src_y + src_h) / tex_h;

        let uv0 = [u_left, v_bottom];
        let uv1 = [u_right, v_bottom];
        let uv2 = [u_left, v_top];
        let uv3 = [u_right, v_top];

        let verts = vec![
            WorldSpriteVertex { pos: p0, uv: uv0, color: rgba },
            WorldSpriteVertex { pos: p1, uv: uv1, color: rgba },
            WorldSpriteVertex { pos: p2, uv: uv2, color: rgba },
            WorldSpriteVertex { pos: p1, uv: uv1, color: rgba },
            WorldSpriteVertex { pos: p3, uv: uv3, color: rgba },
            WorldSpriteVertex { pos: p2, uv: uv2, color: rgba },
        ];

        let data = crate::util::slice_to_bytes(&verts);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("World Sprite Vert Buffer"),
            size: data.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buffer, 0, data);
        self.pass.set_pipeline(&self.world_sprite_pipeline);
        self.pass.set_bind_group(0, bind_group, &[]);
        self.pass.set_bind_group(1, self.camera_bind_group, &[]);
        self.pass.set_vertex_buffer(0, buffer.slice(..data.len() as u64));
        self.pass.draw(0..verts.len() as u32, 0..1);
    }
}
