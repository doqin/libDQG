// ===== Wrapper types (opaque to users, internally wrap wgpu) =====

pub struct ShaderModule(pub(crate) wgpu::ShaderModule);
pub struct RenderPipeline(pub(crate) wgpu::RenderPipeline);
pub struct Buffer(pub(crate) wgpu::Buffer);
pub struct BindGroup(pub(crate) wgpu::BindGroup);
pub struct BindGroupLayout(pub(crate) wgpu::BindGroupLayout);
pub struct Sampler(pub(crate) wgpu::Sampler);
pub struct Texture(pub(crate) wgpu::Texture);
pub struct TextureView(pub(crate) wgpu::TextureView);

// ===== Simple enums =====

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFormat {
    Uint16,
    Uint32,
}
impl IndexFormat {
    pub(crate) fn to_wgpu(self) -> wgpu::IndexFormat {
        match self {
            IndexFormat::Uint16 => wgpu::IndexFormat::Uint16,
            IndexFormat::Uint32 => wgpu::IndexFormat::Uint32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    Float32,
    Float32x2,
    Float32x3,
    Float32x4,
    Sint32,
    Sint32x2,
    Sint32x3,
    Sint32x4,
    Uint32,
    Uint32x2,
    Uint32x3,
    Uint32x4,
}
impl VertexFormat {
    pub(crate) fn to_wgpu(self) -> wgpu::VertexFormat {
        match self {
            VertexFormat::Float32 => wgpu::VertexFormat::Float32,
            VertexFormat::Float32x2 => wgpu::VertexFormat::Float32x2,
            VertexFormat::Float32x3 => wgpu::VertexFormat::Float32x3,
            VertexFormat::Float32x4 => wgpu::VertexFormat::Float32x4,
            VertexFormat::Sint32 => wgpu::VertexFormat::Sint32,
            VertexFormat::Sint32x2 => wgpu::VertexFormat::Sint32x2,
            VertexFormat::Sint32x3 => wgpu::VertexFormat::Sint32x3,
            VertexFormat::Sint32x4 => wgpu::VertexFormat::Sint32x4,
            VertexFormat::Uint32 => wgpu::VertexFormat::Uint32,
            VertexFormat::Uint32x2 => wgpu::VertexFormat::Uint32x2,
            VertexFormat::Uint32x3 => wgpu::VertexFormat::Uint32x3,
            VertexFormat::Uint32x4 => wgpu::VertexFormat::Uint32x4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexStepMode {
    Vertex,
    Instance,
}
impl VertexStepMode {
    pub(crate) fn to_wgpu(self) -> wgpu::VertexStepMode {
        match self {
            VertexStepMode::Vertex => wgpu::VertexStepMode::Vertex,
            VertexStepMode::Instance => wgpu::VertexStepMode::Instance,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}
impl PrimitiveTopology {
    pub(crate) fn to_wgpu(self) -> wgpu::PrimitiveTopology {
        match self {
            PrimitiveTopology::PointList => wgpu::PrimitiveTopology::PointList,
            PrimitiveTopology::LineList => wgpu::PrimitiveTopology::LineList,
            PrimitiveTopology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
            PrimitiveTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
            PrimitiveTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonMode {
    Fill,
    Line,
    Point,
}
impl PolygonMode {
    pub(crate) fn to_wgpu(self) -> wgpu::PolygonMode {
        match self {
            PolygonMode::Fill => wgpu::PolygonMode::Fill,
            PolygonMode::Line => wgpu::PolygonMode::Line,
            PolygonMode::Point => wgpu::PolygonMode::Point,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontFace {
    Ccw,
    Cw,
}
impl FrontFace {
    pub(crate) fn to_wgpu(self) -> wgpu::FrontFace {
        match self {
            FrontFace::Ccw => wgpu::FrontFace::Ccw,
            FrontFace::Cw => wgpu::FrontFace::Cw,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullMode {
    None,
    Front,
    Back,
}
impl CullMode {
    pub(crate) fn to_wgpu(self) -> Option<wgpu::Face> {
        match self {
            CullMode::None => None,
            CullMode::Front => Some(wgpu::Face::Front),
            CullMode::Back => Some(wgpu::Face::Back),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendFactor {
    Zero,
    One,
    Src,
    OneMinusSrc,
    Dst,
    OneMinusDst,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    SrcAlphaSaturated,
    Constant,
    OneMinusConstant,
}
impl BlendFactor {
    pub(crate) fn to_wgpu(self) -> wgpu::BlendFactor {
        match self {
            BlendFactor::Zero => wgpu::BlendFactor::Zero,
            BlendFactor::One => wgpu::BlendFactor::One,
            BlendFactor::Src => wgpu::BlendFactor::Src,
            BlendFactor::OneMinusSrc => wgpu::BlendFactor::OneMinusSrc,
            BlendFactor::Dst => wgpu::BlendFactor::Dst,
            BlendFactor::OneMinusDst => wgpu::BlendFactor::OneMinusDst,
            BlendFactor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
            BlendFactor::DstAlpha => wgpu::BlendFactor::DstAlpha,
            BlendFactor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
            BlendFactor::OneMinusDstAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
            BlendFactor::SrcAlphaSaturated => wgpu::BlendFactor::SrcAlphaSaturated,
            BlendFactor::Constant => wgpu::BlendFactor::Constant,
            BlendFactor::OneMinusConstant => wgpu::BlendFactor::OneMinusConstant,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendOperation {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}
impl BlendOperation {
    pub(crate) fn to_wgpu(self) -> wgpu::BlendOperation {
        match self {
            BlendOperation::Add => wgpu::BlendOperation::Add,
            BlendOperation::Subtract => wgpu::BlendOperation::Subtract,
            BlendOperation::ReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
            BlendOperation::Min => wgpu::BlendOperation::Min,
            BlendOperation::Max => wgpu::BlendOperation::Max,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareFunction {
    Never,
    Less,
    LessEqual,
    Equal,
    GreaterEqual,
    Greater,
    NotEqual,
    Always,
}
impl CompareFunction {
    pub(crate) fn to_wgpu(self) -> wgpu::CompareFunction {
        match self {
            CompareFunction::Never => wgpu::CompareFunction::Never,
            CompareFunction::Less => wgpu::CompareFunction::Less,
            CompareFunction::LessEqual => wgpu::CompareFunction::LessEqual,
            CompareFunction::Equal => wgpu::CompareFunction::Equal,
            CompareFunction::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
            CompareFunction::Greater => wgpu::CompareFunction::Greater,
            CompareFunction::NotEqual => wgpu::CompareFunction::NotEqual,
            CompareFunction::Always => wgpu::CompareFunction::Always,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StencilOperation {
    Keep,
    Zero,
    Replace,
    Invert,
    IncrementClamp,
    DecrementClamp,
    IncrementWrap,
    DecrementWrap,
}
impl StencilOperation {
    pub(crate) fn to_wgpu(self) -> wgpu::StencilOperation {
        match self {
            StencilOperation::Keep => wgpu::StencilOperation::Keep,
            StencilOperation::Zero => wgpu::StencilOperation::Zero,
            StencilOperation::Replace => wgpu::StencilOperation::Replace,
            StencilOperation::Invert => wgpu::StencilOperation::Invert,
            StencilOperation::IncrementClamp => wgpu::StencilOperation::IncrementClamp,
            StencilOperation::DecrementClamp => wgpu::StencilOperation::DecrementClamp,
            StencilOperation::IncrementWrap => wgpu::StencilOperation::IncrementWrap,
            StencilOperation::DecrementWrap => wgpu::StencilOperation::DecrementWrap,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsage {
    Vertex,
    Index,
    Uniform,
    Storage,
    Indirect,
    CopySrc,
    CopyDst,
    MapRead,
    MapWrite,
}
impl BufferUsage {
    pub(crate) fn to_wgpu(self) -> wgpu::BufferUsages {
        match self {
            BufferUsage::Vertex => wgpu::BufferUsages::VERTEX,
            BufferUsage::Index => wgpu::BufferUsages::INDEX,
            BufferUsage::Uniform => wgpu::BufferUsages::UNIFORM,
            BufferUsage::Storage => wgpu::BufferUsages::STORAGE,
            BufferUsage::Indirect => wgpu::BufferUsages::INDIRECT,
            BufferUsage::CopySrc => wgpu::BufferUsages::COPY_SRC,
            BufferUsage::CopyDst => wgpu::BufferUsages::COPY_DST,
            BufferUsage::MapRead => wgpu::BufferUsages::MAP_READ,
            BufferUsage::MapWrite => wgpu::BufferUsages::MAP_WRITE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    R8Unorm,
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Bgra8Unorm,
    Bgra8UnormSrgb,
    Depth32Float,
    Depth24PlusStencil8,
}
impl TextureFormat {
    pub(crate) fn from_wgpu(format: wgpu::TextureFormat) -> Self {
        match format {
            wgpu::TextureFormat::R8Unorm => TextureFormat::R8Unorm,
            wgpu::TextureFormat::Rgba8Unorm => TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb => TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Bgra8Unorm => TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb => TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Depth32Float => TextureFormat::Depth32Float,
            wgpu::TextureFormat::Depth24PlusStencil8 => TextureFormat::Depth24PlusStencil8,
            _ => panic!("Unsupported texture format: {:?}", format),
        }
    }
    pub(crate) fn to_wgpu(self) -> wgpu::TextureFormat {
        match self {
            TextureFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
            TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
            TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8Unorm,
            TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
            TextureFormat::Depth32Float => wgpu::TextureFormat::Depth32Float,
            TextureFormat::Depth24PlusStencil8 => wgpu::TextureFormat::Depth24PlusStencil8,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ColorWriteMask(wgpu::ColorWrites);
impl ColorWriteMask {
    pub const ALL: Self = Self(wgpu::ColorWrites::ALL);
    pub fn to_wgpu(self) -> wgpu::ColorWrites {
        self.0
    }
}

// ===== Descriptor structs =====

pub struct ShaderModuleDescriptor<'a> {
    pub label: Option<&'a str>,
    pub source: &'a str,
}

pub struct BufferDescriptor<'a> {
    pub label: Option<&'a str>,
    pub size: u64,
    pub usage: BufferUsage,
}

pub struct BufferInitDescriptor<'a> {
    pub label: Option<&'a str>,
    pub contents: &'a [u8],
    pub usage: BufferUsage,
}

pub struct RenderPipelineDescriptor<'a> {
    pub label: Option<&'a str>,
    pub vertex: VertexState<'a>,
    pub fragment: Option<FragmentState<'a>>,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub multisample: MultisampleState,
}

pub struct VertexState<'a> {
    pub module: &'a ShaderModule,
    pub entry_point: &'a str,
    pub buffers: &'a [VertexBufferLayout<'a>],
}

pub struct VertexBufferLayout<'a> {
    pub array_stride: u64,
    pub step_mode: VertexStepMode,
    pub attributes: &'a [VertexAttribute],
}

#[derive(Clone, Copy)]
pub struct VertexAttribute {
    pub format: VertexFormat,
    pub offset: u64,
    pub shader_location: u32,
}

pub struct FragmentState<'a> {
    pub module: &'a ShaderModule,
    pub entry_point: &'a str,
    pub targets: &'a [ColorTargetState],
}

#[derive(Clone)]
pub struct ColorTargetState {
    pub format: TextureFormat,
    pub blend: Option<BlendState>,
    pub write_mask: ColorWriteMask,
}

#[derive(Clone)]
pub struct BlendState {
    pub color: BlendComponent,
    pub alpha: BlendComponent,
}

#[derive(Clone, Copy)]
pub struct BlendComponent {
    pub src_factor: BlendFactor,
    pub dst_factor: BlendFactor,
    pub operation: BlendOperation,
}

#[derive(Clone, Copy)]
pub struct PrimitiveState {
    pub topology: PrimitiveTopology,
    pub strip_index_format: Option<IndexFormat>,
    pub front_face: FrontFace,
    pub cull_mode: CullMode,
    pub unclipped_depth: bool,
    pub polygon_mode: PolygonMode,
}
impl Default for PrimitiveState {
    fn default() -> Self {
        Self {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
        }
    }
}

#[derive(Clone, Copy)]
pub struct DepthStencilState {
    pub format: TextureFormat,
    pub depth_write_enabled: bool,
    pub depth_compare: CompareFunction,
    pub stencil: StencilState,
    pub bias: DepthBiasState,
}

#[derive(Clone, Copy)]
pub struct StencilState {
    pub front: StencilFaceState,
    pub back: StencilFaceState,
    pub read_mask: u32,
    pub write_mask: u32,
}
impl Default for StencilState {
    fn default() -> Self {
        Self {
            front: StencilFaceState::default(),
            back: StencilFaceState::default(),
            read_mask: 0xFF,
            write_mask: 0xFF,
        }
    }
}

#[derive(Clone, Copy)]
pub struct StencilFaceState {
    pub compare: CompareFunction,
    pub fail_op: StencilOperation,
    pub depth_fail_op: StencilOperation,
    pub pass_op: StencilOperation,
}
impl Default for StencilFaceState {
    fn default() -> Self {
        Self {
            compare: CompareFunction::Always,
            fail_op: StencilOperation::Keep,
            depth_fail_op: StencilOperation::Keep,
            pass_op: StencilOperation::Keep,
        }
    }
}

#[derive(Clone, Copy)]
pub struct DepthBiasState {
    pub bias: i32,
    pub slope_scale: f32,
    pub clamp: f32,
}
impl Default for DepthBiasState {
    fn default() -> Self {
        Self { bias: 0, slope_scale: 0.0, clamp: 0.0 }
    }
}

#[derive(Clone, Copy)]
pub struct MultisampleState {
    pub count: u32,
    pub mask: u64,
    pub alpha_to_coverage_enabled: bool,
}
impl Default for MultisampleState {
    fn default() -> Self {
        Self { count: 1, mask: !0, alpha_to_coverage_enabled: false }
    }
}
