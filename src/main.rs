#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate image;

use gfx::traits::FactoryExt;
use gfx::Device;
use gfx::Factory;

use glutin::GlContext;

type ColourFormat = gfx::format::Srgba8;
type DepthFormat = gfx::format::DepthStencil;
type Resources = gfx_device_gl::Resources;

const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];
const QUAD_COORDS: [[f32; 2]; 4] = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];

const NUM_IMAGE_INSTANCES: usize = 3;

gfx_vertex_struct!(QuadCorner {
    corner_zero_to_one: [f32; 2] = "a_CornerZeroToOne",
});

gfx_vertex_struct!(ImageInstance {
    position_within_window_in_pixels: [f32; 2] = "a_PositionWithinWindowInPixels",
    size_in_pixels: [f32; 2] = "a_SizeInPixels",
});

gfx_constant_struct!(Properties {
    window_size_in_pixels: [f32; 2] = "u_WindowSizeInPixels",
});

gfx_pipeline!(pipe {
    quad_corners: gfx::VertexBuffer<QuadCorner> = (),
    image_instances: gfx::InstanceBuffer<ImageInstance> = (),
    properties: gfx::ConstantBuffer<Properties> = "Properties",
    texture: gfx::TextureSampler<[f32; 4]> = "t_Texture",
    out_colour: gfx::BlendTarget<ColourFormat> =
        ("Target0", gfx::state::ColorMask::all(), gfx::preset::blend::ALPHA),
    out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
});

fn create_instance_buffer<R, F, T>(
    size: usize,
    factory: &mut F,
) -> Result<gfx::handle::Buffer<R, T>, gfx::buffer::CreationError>
where
    R: gfx::Resources,
    F: gfx::Factory<R> + gfx::traits::FactoryExt<R>,
{
    factory.create_buffer(
        size,
        gfx::buffer::Role::Vertex,
        gfx::memory::Usage::Data,
        gfx::memory::Bind::TRANSFER_DST,
    )
}

fn main() {
    let builder = glutin::WindowBuilder::new().with_dimensions(960, 720);
    let mut events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let (window, mut device, mut factory, rtv, dsv) =
        gfx_window_glutin::init::<ColourFormat, DepthFormat>(builder, context, &events_loop);

    use gfx::texture as t;
    let test_pattern = image::load_from_memory(include_bytes!("images/test_pattern.png"))
        .expect("Failed to decode test pattern")
        .to_rgba();
    let (img_width, img_height) = test_pattern.dimensions();
    let tex_kind = t::Kind::D2(img_width as u16, img_height as u16, t::AaMode::Single);
    let tex_mipmap = t::Mipmap::Allocated;
    let (_, texture_srv) = factory
        .create_texture_immutable_u8::<ColourFormat>(tex_kind, tex_mipmap, &[&test_pattern])
        .expect("Failed to create texture");

    let sampler = factory.create_sampler(t::SamplerInfo::new(
        t::FilterMethod::Scale,
        t::WrapMode::Tile,
    ));

    let pso = factory
        .create_pipeline_simple(
            include_bytes!("shaders/shader.150.vert"),
            include_bytes!("shaders/shader.150.frag"),
            pipe::new(),
        )
        .expect("Failed to create pipeline");
    let mut encoder: gfx::Encoder<Resources, gfx_device_gl::CommandBuffer> =
        factory.create_command_buffer().into();

    let quad_corners_data = QUAD_COORDS
        .iter()
        .map(|v| QuadCorner {
            corner_zero_to_one: *v,
        })
        .collect::<Vec<_>>();

    let (quad_corners_buf, slice) =
        factory.create_vertex_buffer_with_slice(&quad_corners_data, &QUAD_INDICES[..]);

    let data = pipe::Data {
        quad_corners: quad_corners_buf,
        image_instances: create_instance_buffer(NUM_IMAGE_INSTANCES, &mut factory)
            .expect("Failed to create instance buffer"),
        properties: factory.create_constant_buffer(1),
        texture: (texture_srv, sampler),
        out_colour: rtv,
        out_depth: dsv,
    };
    let mut bundle = gfx::pso::bundle::Bundle::new(slice, pso, data);

    let (window_width, window_height, _, _) = bundle.data.out_colour.get_dimensions();
    let properties = Properties {
        window_size_in_pixels: [window_width as f32, window_height as f32],
    };

    bundle.slice.instances = Some((NUM_IMAGE_INSTANCES as u32, 0));
    let image_instances_upload: gfx::handle::Buffer<Resources, ImageInstance> = factory
        .create_upload_buffer(NUM_IMAGE_INSTANCES)
        .expect("Failed to create instance upload buffer");
    {
        let mut image_instance_writer = factory
            .write_mapping(&image_instances_upload)
            .expect("Failed to map upload buffer");

        image_instance_writer[0].position_within_window_in_pixels = [0., 0.];
        image_instance_writer[0].size_in_pixels = [300., 300.];
        image_instance_writer[1].position_within_window_in_pixels = [300., 500.];
        image_instance_writer[1].size_in_pixels = [200., 500.];
        image_instance_writer[2].position_within_window_in_pixels = [600., 300.];
        image_instance_writer[2].size_in_pixels = [300., 100.];
    }
    encoder.update_constant_buffer(&bundle.data.properties, &properties);

    let mut running = true;
    while running {
        encoder.clear(&bundle.data.out_colour, [0.0, 0.0, 0.0, 1.0]);
        encoder.clear_depth(&bundle.data.out_depth, 1.0);

        encoder
            .copy_buffer(
                &image_instances_upload,
                &bundle.data.image_instances,
                0,
                0,
                NUM_IMAGE_INSTANCES,
            )
            .expect("Failed to copy instances");
        bundle.encode(&mut encoder);

        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();

        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => {
                    running = false;
                }
                _ => (),
            },
            _ => (),
        });
    }
}
