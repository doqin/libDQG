use std::ops::Range;
use crate::types::Color;
use crate::renderer::types::{
    Buffer, BindGroup, IndexFormat, RenderPipeline,
};

pub struct DrawPass<'a> {
    pub(crate) pass: wgpu::RenderPass<'a>,
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) immediate_pipeline: &'a RenderPipeline,
    pub(crate) screen_w: u32,
    pub(crate) screen_h: u32,
}

impl DrawPass<'_> {
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.pass.draw(vertices, instances);
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.pass.draw_indexed(indices, base_vertex, instances);
    }

    pub fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32) {
        self.pass.set_viewport(x, y, w, h, min_depth, max_depth);
    }

    pub fn set_scissor_rect(&mut self, x: u32, y: u32, w: u32, h: u32) {
        self.pass.set_scissor_rect(x, y, w, h);
    }

    pub fn set_blend_constant(&mut self, color: Color) {
        self.pass.set_blend_constant(color.as_wgpu_color());
    }

    pub fn set_stencil_reference(&mut self, reference: u32) {
        self.pass.set_stencil_reference(reference);
    }

    pub fn set_pipeline(&mut self, pipeline: &RenderPipeline) {
        self.pass.set_pipeline(&pipeline.0);
    }

    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: &Buffer, offset: u64, size: u64) {
        self.pass.set_vertex_buffer(slot, buffer.0.slice(offset..offset + size));
    }

    pub fn set_index_buffer(&mut self, buffer: &Buffer, format: IndexFormat, offset: u64, size: u64) {
        self.pass.set_index_buffer(buffer.0.slice(offset..offset + size), format.to_wgpu());
    }

    pub fn set_bind_group(&mut self, group: u32, bind_group: &BindGroup, offsets: &[u32]) {
        self.pass.set_bind_group(group, &bind_group.0, offsets);
    }

    pub fn push_debug_group(&mut self, label: &str) {
        self.pass.push_debug_group(label);
    }

    pub fn pop_debug_group(&mut self) {
        self.pass.pop_debug_group();
    }

    pub fn insert_debug_marker(&mut self, label: &str) {
        self.pass.insert_debug_marker(label);
    }
}
