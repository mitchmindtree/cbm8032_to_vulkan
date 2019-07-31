use nannou::image;
use nannou::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const CHAR_SHEET_FILE_NAME: &str = "faces_16x16.jpg";
const CHAR_SHEET_ROWS: u8 = 16;
const CHAR_SHEET_COLS: u8 = 16;

const CHARS_PER_LINE: u8 = 80;
const LINES: u8 = 25;

/// Items related to the visualisation.
pub struct Vis {
    char_sheet: Arc<vk::ImmutableImage<vk::Format>>,
    graphics: Graphics,
}

// The vulkan renderpass, pipeline and related items.
struct Graphics {
    render_pass: Arc<dyn vk::RenderPassAbstract + Send + Sync>,
    pipeline: Arc<dyn vk::GraphicsPipelineAbstract + Send + Sync>,
    vertex_buffer: Arc<vk::CpuAccessibleBuffer<[Vertex]>>,
    view_fbo: RefCell<ViewFbo>,
    sampler: Arc<vk::Sampler>,
    descriptor_set: Arc<dyn vk::DescriptorSet + Send + Sync>,
}

// Vertex type used for GPU geometry.
#[derive(Clone, Copy, Debug, Default)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

// InstanceData is the vertex type that describes the unique data per instance.
#[derive(Clone, Copy, Debug, Default)]
struct InstanceData {
    position_offset: [f32; 2],
    tex_coords_offset: [f32; 2],
}

vk::impl_vertex!(Vertex, position, tex_coords);
vk::impl_vertex!(InstanceData, position_offset, tex_coords_offset);

/// Initialise the state of the visualisation.
pub fn init(assets_path: &Path, queue: Arc<vk::Queue>, msaa_samples: u32) -> Vis {
    let char_sheet = load_char_sheet(assets_path, queue.clone());
    let graphics = init_graphics(queue, msaa_samples, &char_sheet);
    Vis {
        char_sheet,
        graphics,
    }
}

/// Draw the visualisation to the `Frame`.
pub fn view(vis: &Vis, frame: &Frame) {}

/// Select the best GPU from those available.
pub fn best_gpu(app: &App) -> Option<vk::PhysicalDevice> {
    find_discrete_gpu(app.vk_physical_devices()).or_else(|| app.default_vk_physical_device())
}

// Load the character sheet.
fn load_char_sheet(
    assets_path: &Path,
    queue: Arc<vk::Queue>,
) -> Arc<vk::ImmutableImage<vk::Format>> {
    let images_path = images_path(assets_path);
    let path = images_path.join(CHAR_SHEET_FILE_NAME);
    let image = image::open(&path).expect("failed to open image");
    let rgb_image = image.to_rgb();
    let (width, height) = rgb_image.dimensions();
    let raw_image = rgb_image.into_raw();
    let image_data = raw_image.into_iter();
    let dims = vk::image::Dimensions::Dim2d { width, height };
    let format = vk::Format::R8G8B8Srgb;
    let (image, img_fut) = vk::ImmutableImage::from_iter(image_data, dims, format, queue)
        .expect("failed to load character sheet onto GPU");
    img_fut
        .then_signal_fence_and_flush()
        .expect("failed to signal fence and flush `img_fut`")
        .wait(None)
        .expect("failed to wait for the `img_fut`");
    image
}

// Initialise the vulkan graphics state.
fn init_graphics(
    queue: Arc<vk::Queue>,
    msaa_samples: u32,
    char_sheet: &Arc<vk::ImmutableImage<vk::Format>>,
) -> Graphics {
    let device = queue.device().clone();
    let color_format = nannou::frame::COLOR_FORMAT;
    let render_pass = create_render_pass(device.clone(), color_format, msaa_samples);
    let pipeline = create_pipeline(render_pass.clone());
    let vertex_buffer = create_vertex_buffer(device.clone());
    let view_fbo = RefCell::new(Default::default());
    let sampler = vk::SamplerBuilder::new()
        .build(device)
        .expect("failed to build sampler");
    let descriptor_set =
        create_descriptor_set(pipeline.clone(), char_sheet.clone(), sampler.clone());
    Graphics {
        render_pass,
        pipeline,
        vertex_buffer,
        view_fbo,
        sampler,
        descriptor_set,
    }
}

// The render pass used for the graphics pipeline.
fn create_render_pass(
    device: Arc<vk::Device>,
    color_format: vk::Format,
    msaa_samples: u32,
) -> Arc<dyn vk::RenderPassAbstract + Send + Sync> {
    let rp = vk::single_pass_renderpass!(
        device,
        attachments: {
            color: {
                load: Load,
                store: Store,
                format: color_format,
                samples: msaa_samples,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    )
    .expect("failed to create renderpass");
    Arc::new(rp)
}

// Create the graphics pipeline for running the shaders.
fn create_pipeline<R>(render_pass: R) -> Arc<dyn vk::GraphicsPipelineAbstract + Send + Sync>
where
    R: vk::RenderPassAbstract + Send + Sync + 'static,
{
    let device = render_pass.device().clone();
    let vs = vs::Shader::load(device.clone()).expect("failed to load vertex shader");
    let fs = fs::Shader::load(device.clone()).expect("failed to load fragment shader");
    let subpass = vk::Subpass::from(render_pass, 0).expect("no subpass for `id`");
    let pipeline = vk::GraphicsPipeline::start()
        //.sample_shading_enabled(1.0)
        .vertex_input(vk::OneVertexOneInstanceDefinition::<Vertex, InstanceData>::new())
        .vertex_shader(vs.main_entry_point(), ())
        .triangle_list()
        .viewports_dynamic_scissors_irrelevant(1)
        .fragment_shader(fs.main_entry_point(), ())
        .blend_alpha_blending()
        .render_pass(subpass)
        .build(device)
        .expect("failed to create graphics pipeline");
    Arc::new(pipeline)
}

// Create a vertex buffer containing the two triangles that make up a single character slot.
fn create_vertex_buffer(device: Arc<vk::Device>) -> Arc<vk::CpuAccessibleBuffer<[Vertex]>> {
    // Vertex position range:
    // - left to right: -1.0 to 1.0
    // - bottom to top: 1.0 to -1.0
    let p_w = 2.0 / CHARS_PER_LINE as f32;
    let p_h = 2.0 / LINES as f32;
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

    let usage = vk::BufferUsage::all();
    let vertex_buffer = vk::CpuAccessibleBuffer::from_iter(device, usage, vs.iter().cloned())
        .expect("failed to construct vertex buffer");
    vertex_buffer
}

// Create the descriptor set with an image sampler for sampling the character sheet.
fn create_descriptor_set(
    pipeline: Arc<dyn vk::GraphicsPipelineAbstract + Send + Sync>,
    char_sheet: Arc<vk::ImmutableImage<vk::Format>>,
    sampler: Arc<vk::Sampler>,
) -> Arc<dyn vk::DescriptorSet + Send + Sync> {
    let descriptor_set = vk::PersistentDescriptorSet::start(pipeline.clone(), 0)
        .add_sampled_image(char_sheet.clone(), sampler.clone())
        .expect("failed to add character sheet sampler to the descriptor set")
        .build()
        .expect("failed to build the descriptor set");
    Arc::new(descriptor_set)
}

// Directory in which images are stored.
fn images_path(assets: &Path) -> PathBuf {
    assets.join("images")
}

// Return a dedicated GPU device if there is one.
fn find_discrete_gpu<'a, I>(devices: I) -> Option<vk::PhysicalDevice<'a>>
where
    I: IntoIterator<Item = vk::PhysicalDevice<'a>>,
{
    devices
        .into_iter()
        .find(|d| d.ty() == vk::PhysicalDeviceType::DiscreteGpu)
}

mod vs {
    nannou::vulkano_shaders::shader! {
        ty: "vertex",
        src: "
#version 450

// The quad vertex positions.
layout(location = 0) in vec2 position;
layout(location = 1) in vec2 tex_coords;

// The per-instance data.
layout(location = 2) in vec2 position_offset;
layout(location = 3) in vec2 tex_coords_offset;

// Feed the offset texture coordinatees through to the frag shader.
layout(location = 0) out vec2 v_tex_coords;

void main() {
    // Apply the tex coord offset for the instance.
    v_tex_coords = tex_coords + tex_coords_offset;
    // Apply the position offset for the instance.
    gl_Position = vec4(position + position_offset, 0.0, 1.0);
}"
    }
}

mod fs {
    nannou::vulkano_shaders::shader! {
        ty: "fragment",
        src: "
#version 450

layout(location = 0) in vec2 v_tex_coords;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D char_sheet;

void main() {
    f_color = texture(char_sheet, v_tex_coords);
}
"
    }
}
