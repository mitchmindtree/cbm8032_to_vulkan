//! Items related to the visualisation including vulkan graphics and character sheet logic.

use crate::conf::Config;
use nannou::image;
use nannou::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

const CHAR_SHEET_FILE_NAME: &str = "PetASCII_Combined.png";
const CHAR_SHEET_ROWS: u8 = 32;
const CHAR_SHEET_COLS: u8 = 16;
const CHARS_PER_LINE: u8 = 80;
const DATA_LINES: u8 = 25;
const BLANK_LINES: u8 = 2;
const TOTAL_LINES: u8 = DATA_LINES + BLANK_LINES;
const GRAPHICS_MODE_ROW_OFFSET: u8 = 0;
const TEXT_MODE_ROW_OFFSET: u8 = 16;
const VERTEX_COUNT: usize = 6;
const DECAY_IMAGE_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

pub const CBM_8032_FRAME_DATA_LEN: usize = CHARS_PER_LINE as usize * DATA_LINES as usize;

/// Items related to the visualisation.
pub struct Vis {
    _char_sheet: wgpu::Texture,
    char_sheet_view: wgpu::TextureView,
    graphics: RefCell<Graphics>,
}

/// The frame type representing all data necessary for displaying a single frame.
pub struct Cbm8032Frame {
    pub mode: Cbm8032FrameMode,
    pub data: Box<Cbm8032FrameData>,
}

/// The two modes in which
#[derive(Clone, Copy, Debug)]
pub enum Cbm8032FrameMode {
    Graphics,
    Text,
}

/// The type used to represent the CBM 8032 graphical data.
pub type Cbm8032FrameData = [u8; CBM_8032_FRAME_DATA_LEN];

// The vulkan renderpass, pipeline and related items.
struct Graphics {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    decay: Decay,
    _sampler: wgpu::Sampler,
}

struct Decay {
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Uniforms {
    colouration: [f32; 4],
    sustain: f32,
}

// Vertex type used for GPU geometry.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

// Instance is the vertex type that describes the unique data per instance.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct Instance {
    position_offset: [f32; 2],
    tex_coords_offset: [f32; 2],
}

impl Cbm8032Frame {
    const BLANK_BYTE: u8 = 32;
    const BLANK_DATA: Cbm8032FrameData = [Self::BLANK_BYTE; CBM_8032_FRAME_DATA_LEN];

    /// Construct a new `Cbm8032Frame` from the given mode and data.
    pub fn new(mode: Cbm8032FrameMode, data: Box<Cbm8032FrameData>) -> Self {
        Cbm8032Frame {
            mode,
            data,
        }
    }

    /// Create a frame containing blank data in graphics mode.
    pub fn blank_graphics() -> Self {
        let data = Box::new(Self::BLANK_DATA);
        Self::new(Cbm8032FrameMode::Graphics, data)
    }

    /// Create a frame containing random data in graphics mode.
    pub fn _random_graphics() -> Self {
        let mut frame = Self::blank_graphics();
        randomise_frame_data(&mut frame.data);
        frame
    }

    pub fn _test_graphics() -> Self {
        let mut frame = Self::_random_graphics();
        let data = [27u8; 16];
        frame.data[..16].copy_from_slice(&data);
        frame
    }
}

/// Randomise the given frame data.
pub fn randomise_frame_data(data: &mut Cbm8032FrameData) {
    for b in data.iter_mut() {
        *b = random();
    }
}

/// Initialise the state of the visualisation.
pub fn init(assets_path: &Path, window: &nannou::window::Window, msaa_samples: u32) -> Vis {
    let char_sheet = load_char_sheet(assets_path, window);
    let char_sheet_view = char_sheet.view().build();
    let device = window.swap_chain_device();
    let (w, h) = window.inner_size_pixels();
    let graphics = RefCell::new(init_graphics(device, [w, h], msaa_samples, &char_sheet_view));
    Vis {
        _char_sheet: char_sheet,
        char_sheet_view,
        graphics,
    }
}

/// Draw the visualisation to the `Frame`.
pub fn view(config: &Config, vis: &Vis, cbm_frame: &Cbm8032Frame, frame: Frame) {
    let device_queue_pair = frame.device_queue_pair();
    let device = device_queue_pair.device();

    // Update the uniforms.
    let hsv = config.colouration.hsv();
    let lin_srgb: LinSrgb = hsv.into();
    let colouration = [lin_srgb.red, lin_srgb.green, lin_srgb.blue, config.colouration.alpha];
    let sustain = config.sustain;
    let uniforms = Uniforms { colouration, sustain };
    let uniforms_size = std::mem::size_of::<Uniforms>() as wgpu::BufferAddress;
    let uniforms_bytes = uniforms_as_bytes(&uniforms);
    let usage = wgpu::BufferUsage::COPY_SRC;
    let new_uniform_buffer = device.create_buffer_with_data(uniforms_bytes, usage);

    // Create the instance data buffer.
    fn blank_line_bytes() -> impl Iterator<Item = u8> {
        (0..CHARS_PER_LINE).map(|_| Cbm8032Frame::BLANK_BYTE)
    }
    let all_bytes = blank_line_bytes()
        .chain(cbm_frame.data.iter().cloned())
        .chain(blank_line_bytes());
    let instances: Vec<Instance> = all_bytes
        .enumerate()
        .map(|(ix, byte)| {
            let col_row = byte_to_char_sheet_col_row(byte, &cbm_frame.mode);
            let tex_coords_offset = char_sheet_col_row_to_tex_coords_offset(col_row);
            let position_offset = serial_char_index_to_position_offset(ix as _);
            Instance {
                position_offset,
                tex_coords_offset,
            }
        })
        .collect();
    let instances_bytes = instances_as_bytes(&instances[..]);
    let usage = wgpu::BufferUsage::VERTEX;
    let instance_buffer = device.create_buffer_with_data(instances_bytes, usage);

    // If the window changed sizes, we need to recreate the decay buffer and in turn, the whole
    // graphics pipeline.
    let frame_wh = frame.texture_size();
    let frame_msaa_samples = frame.texture_msaa_samples();
    if vis.graphics.borrow().decay.texture_view.size() != frame.texture_size() {
        let new_graphics = init_graphics(device, frame_wh, frame_msaa_samples, &vis.char_sheet_view);
        vis.graphics.replace(new_graphics);
    }

    // Encode the new buffer copies and the render pass.
    let mut encoder = frame.command_encoder();
    let graphics = vis.graphics.borrow();
    encoder.copy_buffer_to_buffer(&new_uniform_buffer, 0, &graphics.uniform_buffer, 0, uniforms_size);

    // Render pass for rendering to the decay image.
    {
        let decay = &graphics.decay;
        let load_op = wgpu::LoadOp::Load;
        let clear_color = wgpu::Color::TRANSPARENT;
        let mut render_pass = wgpu::RenderPassBuilder::new()
            .color_attachment(&decay.texture_view, |color| {
                color
                    .load_op(load_op)
                    .clear_color(clear_color)
            })
            .begin(&mut encoder);
        render_pass.set_bind_group(0, &decay.bind_group, &[]);
        render_pass.set_pipeline(&decay.pipeline);
        render_pass.set_vertex_buffer(0, &graphics.vertex_buffer, 0, 0);
        render_pass.set_vertex_buffer(1, &instance_buffer, 0, 0);
        let vertex_range = 0..VERTEX_COUNT as u32;
        let instance_range = 0..instances.len() as u32;
        render_pass.draw(vertex_range, instance_range);
    }

    // Render pass for rendering to the swapchain image.
    {
        let mut render_pass = wgpu::RenderPassBuilder::new()
            .color_attachment(frame.texture_view(), |color| color)
            .begin(&mut encoder);
        render_pass.set_bind_group(0, &graphics.bind_group, &[]);
        render_pass.set_pipeline(&graphics.pipeline);
        render_pass.set_vertex_buffer(0, &graphics.vertex_buffer, 0, 0);
        render_pass.set_vertex_buffer(1, &instance_buffer, 0, 0);
        let vertex_range = 0..VERTEX_COUNT as u32;
        let instance_range = 0..instances.len() as u32;
        render_pass.draw(vertex_range, instance_range);
    }
}

/// Given a byte value from the serial data, return the column and row of the character within the
/// `CHAR_SHEET` starting from the top left.
pub fn byte_to_char_sheet_col_row(byte: u8, mode: &Cbm8032FrameMode) -> [u8; 2] {
    let row_offset = match mode {
        Cbm8032FrameMode::Graphics => GRAPHICS_MODE_ROW_OFFSET,
        Cbm8032FrameMode::Text => TEXT_MODE_ROW_OFFSET,
    };
    let col = byte % CHAR_SHEET_COLS;
    let row = row_offset + byte / (CHAR_SHEET_ROWS / 2);
    [col, row]
}

/// Given a column and row within the char sheet starting from the top left, produce the tex coords
/// offset for that character.
pub fn char_sheet_col_row_to_tex_coords_offset([col, row]: [u8; 2]) -> [f32; 2] {
    let x = col as f32 / CHAR_SHEET_COLS as f32;
    let y = row as f32 / CHAR_SHEET_ROWS as f32;
    [x, y]
}

/// Given the index of a character within the serial data, produce the position offset for the
/// character.
pub fn serial_char_index_to_position_offset(char_index: u16) -> [f32; 2] {
    let col = char_index % CHARS_PER_LINE as u16;
    let row = char_index / CHARS_PER_LINE as u16;
    let x = 2.0 * col as f32 / CHARS_PER_LINE as f32;
    let y = 2.0 * row as f32 / TOTAL_LINES as f32;
    [x, y]
}

// Load the character sheet.
fn load_char_sheet(assets_path: &Path, window: &nannou::window::Window) -> wgpu::Texture {
    let images_path = images_path(assets_path);
    let path = images_path.join(CHAR_SHEET_FILE_NAME);
    let image = image::open(&path).expect("failed to open image");
    // Load the image as a texture.
    wgpu::Texture::from_image(window, &image)
}

// Initialise the WGPU graphics state.
fn init_graphics(
    device: &wgpu::Device,
    swap_chain_dims: [u32; 2],
    msaa_samples: u32,
    char_sheet: &wgpu::TextureView,
) -> Graphics {
    // Load shader modules.
    let vs_mod = wgpu::shader_from_spirv_bytes(device, include_bytes!("glsl/vert.spv"));
    let fs_mod = wgpu::shader_from_spirv_bytes(device, include_bytes!("glsl/frag.spv"));

    // Initialise the uniform buffer.
    let colouration = [0.0; 4];
    let sustain = 1.0;
    let uniforms = Uniforms { colouration, sustain };
    let uniforms_bytes = uniforms_as_bytes(&uniforms);
    let usage = wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST;
    let uniform_buffer = device.create_buffer_with_data(uniforms_bytes, usage);

    // For sampling the char sheet.
    let sampler = create_sampler(device);

    let decay = init_decay(
        device,
        swap_chain_dims,
        &vs_mod,
        char_sheet,
        &uniform_buffer,
        &sampler,
    );

    let bind_group_layout = create_bind_group_layout(
        device,
        char_sheet.component_type(),
        decay.texture.component_type(),
    );
    let bind_group = create_bind_group(
        device,
        &bind_group_layout,
        &uniform_buffer,
        char_sheet,
        &decay.texture_view,
        &sampler,
    );
    let pipeline_layout = create_pipeline_layout(device, &bind_group_layout);
    let pipeline = create_pipeline(
        device,
        &pipeline_layout,
        &vs_mod,
        &fs_mod,
        Frame::TEXTURE_FORMAT,
        msaa_samples,
    );

    let vertex_buffer = create_vertex_buffer(device.clone());

    Graphics {
        pipeline,
        bind_group,
        vertex_buffer,
        uniform_buffer,
        decay,
        _sampler: sampler,
    }
}

fn init_decay(
    device: &wgpu::Device,
    swap_chain_dims: [u32; 2],
    vs_mod: &wgpu::ShaderModule,
    char_sheet: &wgpu::TextureView,
    uniform_buffer: &wgpu::Buffer,
    sampler: &wgpu::Sampler,
) -> Decay {
    let fs_mod = wgpu::shader_from_spirv_bytes(device, include_bytes!("glsl/decay_frag.spv"));
    let texture = wgpu::TextureBuilder::new()
        .size(swap_chain_dims)
        .usage(wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED)
        .format(DECAY_IMAGE_TEXTURE_FORMAT)
        .build(device);
    let texture_view = texture.view().build();
    let bind_group_layout = create_decay_bind_group_layout(device, char_sheet.component_type());
    let bind_group = create_decay_bind_group(device, &bind_group_layout, &uniform_buffer, char_sheet, &sampler);
    let pipeline_layout = create_pipeline_layout(device, &bind_group_layout);
    let msaa_samples = 1;
    let pipeline = create_pipeline(
        device,
        &pipeline_layout,
        &vs_mod,
        &fs_mod,
        texture_view.format(),
        msaa_samples,
    );
    Decay {
        texture,
        texture_view,
        bind_group,
        pipeline,
    }
}

fn create_bind_group_layout(
    device: &wgpu::Device,
    char_sheet_texture_component_type: wgpu::TextureComponentType,
    decay_texture_component_type: wgpu::TextureComponentType,
) -> wgpu::BindGroupLayout {
    wgpu::BindGroupLayoutBuilder::new()
        .uniform_buffer(wgpu::ShaderStage::FRAGMENT, false)
        .sampled_texture(
            wgpu::ShaderStage::FRAGMENT,
            false,
            wgpu::TextureViewDimension::D2,
            char_sheet_texture_component_type,
        )
        .sampled_texture(
            wgpu::ShaderStage::FRAGMENT,
            false,
            wgpu::TextureViewDimension::D2,
            decay_texture_component_type,
        )
        .sampler(wgpu::ShaderStage::FRAGMENT)
        .build(device)
}

fn create_decay_bind_group_layout(
    device: &wgpu::Device,
    texture_component_type: wgpu::TextureComponentType,
) -> wgpu::BindGroupLayout {
    wgpu::BindGroupLayoutBuilder::new()
        .uniform_buffer(wgpu::ShaderStage::FRAGMENT, false)
        .sampled_texture(
            wgpu::ShaderStage::FRAGMENT,
            false,
            wgpu::TextureViewDimension::D2,
            texture_component_type,
        )
        .sampler(wgpu::ShaderStage::FRAGMENT)
        .build(device)
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
    char_sheet_texture_view: &wgpu::TextureView,
    decay_texture_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    wgpu::BindGroupBuilder::new()
        .buffer::<Uniforms>(uniform_buffer, 0..1)
        .texture_view(char_sheet_texture_view)
        .texture_view(decay_texture_view)
        .sampler(sampler)
        .build(device, layout)
}

fn create_decay_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
    char_sheet_texture_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    wgpu::BindGroupBuilder::new()
        .buffer::<Uniforms>(uniform_buffer, 0..1)
        .texture_view(char_sheet_texture_view)
        .sampler(sampler)
        .build(device, layout)
}

fn create_pipeline_layout(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    let desc = wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
    };
    device.create_pipeline_layout(&desc)
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    vs_mod: &wgpu::ShaderModule,
    fs_mod: &wgpu::ShaderModule,
    dst_format: wgpu::TextureFormat,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    wgpu::RenderPipelineBuilder::from_layout(layout, vs_mod)
        .fragment_shader(&fs_mod)
        .color_format(dst_format)
        .add_vertex_buffer::<Vertex>(&wgpu::vertex_attr_array![
            0 => Float2,
            1 => Float2
        ])
        .add_instance_buffer::<Instance>(&wgpu::vertex_attr_array![
            2 => Float2,
            3 => Float2
        ])
        .sample_count(sample_count)
        .build(device)
}

// Create a vertex buffer containing the two triangles that make up a single character slot.
fn create_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    // Vertex position range:
    // - left to right: -1.0 to 1.0
    // - bottom to top: -1.0 to 1.0
    let p_w = 2.0 / CHARS_PER_LINE as f32;
    let p_h = 2.0 / TOTAL_LINES as f32;
    let p_tl = [-1.0, -1.0];
    let p_tr = [-1.0 + p_w, -1.0];
    let p_bl = [-1.0, -1.0 + p_h];
    let p_br = [-1.0 + p_w, -1.0 + p_h];

    // Texture coordinates range:
    // - left to right: 0.0 to 1.0
    // - bottom to top: 1.0 to 0.0
    let tc_w = 1.0 / CHAR_SHEET_COLS as f32;
    let tc_h = 1.0 / CHAR_SHEET_ROWS as f32;
    let tc_tl = [0.0, 0.0];
    let tc_tr = [tc_w, 0.0];
    let tc_bl = [0.0, tc_h];
    let tc_br = [tc_w, tc_h];

    // Vertices for each corner of the rect in the very top left of the visualisation.
    let v = |position, tex_coords| Vertex {
        position,
        tex_coords,
    };
    let tl = v(p_tl, tc_tl);
    let tr = v(p_tr, tc_tr);
    let bl = v(p_bl, tc_bl);
    let br = v(p_br, tc_br);

    // The two triangles that make up the rectangle.
    let vs = [tl, tr, br, tl, br, bl];

    assert_eq!(vs.len(), VERTEX_COUNT);

    let vertices_bytes = vertices_as_bytes(&vs[..]);
    let usage = wgpu::BufferUsage::VERTEX;
    device.create_buffer_with_data(vertices_bytes, usage)
}

// Create the sampler used for sampling the character sheet image in the fragment shader.
fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    wgpu::SamplerBuilder::new()
        .mag_filter(wgpu::FilterMode::Nearest)
        .min_filter(wgpu::FilterMode::Nearest)
        .build(device)
}

// Directory in which images are stored.
fn images_path(assets: &Path) -> PathBuf {
    assets.join("images")
}

// See the `nannou::wgpu::bytes` documentation for why the following are necessary.

fn vertices_as_bytes(data: &[Vertex]) -> &[u8] {
    unsafe { wgpu::bytes::from_slice(data) }
}

fn uniforms_as_bytes(uniforms: &Uniforms) -> &[u8] {
    unsafe { wgpu::bytes::from(uniforms) }
}

fn instances_as_bytes(data: &[Instance]) -> &[u8] {
    unsafe { wgpu::bytes::from_slice(data) }
}
