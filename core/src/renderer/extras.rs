use crate::renderer::DrawPass;
use crate::renderer::types::RenderPipeline;
use crate::types::Color;

const VS_SRC: &str = r#"
struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@location(0) pos: vec2<f32>, @location(1) color: vec4<f32>) -> VOut {
    var out: VOut;
    out.pos = vec4<f32>(pos, 0.0, 1.0);
    out.color = color;
    return out;
}
"#;

const FS_SRC: &str = r#"
@fragment
fn fs_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
    return color;
}
"#;

#[repr(C)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
}

pub fn create_shape_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> RenderPipeline {
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
            buffers: &[wgpu::VertexBufferLayout {
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
            }],
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
        depth_stencil: None,
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
    RenderPipeline(wgpu_pipeline)
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
        let data = unsafe {
            std::slice::from_raw_parts(
                verts.as_ptr() as *const u8,
                verts.len() * std::mem::size_of::<Vertex>(),
            )
        };
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shape Vert Buffer"),
            size: data.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buffer, 0, data);
        self.pass.set_pipeline(&self.immediate_pipeline.0);
        self.pass.set_vertex_buffer(0, buffer.slice(..data.len() as u64));
        self.pass.draw(0..verts.len() as u32, 0..1);
    }
}
