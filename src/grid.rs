use encase::private::AsRefMatrixParts;
use wgpu::util::DeviceExt;


pub struct GridRenderRoutine {
    pipeline: wgpu::RenderPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
}

impl GridRenderRoutine {
    pub fn new(
        renderer: &rend3::Renderer,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        // Creater shader module
        let shader_module = renderer.device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("grid shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("grid.wgsl").into()),
            }
        );

        // Set up uniform buffer layout
        let uniform_bind_group_layout =
            renderer.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX
                            | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("grid bind group layout"),
                });

        // Create pipeline
        let render_pipeline_layout =
            renderer.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("grid render pipeline layout"),
                    bind_group_layouts: &[&uniform_bind_group_layout],
                    push_constant_ranges: &[],
                });
        let pipeline =
            renderer.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("grid render pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader_module,
                        entry_point: "vs_main",
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader_module,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: surface_format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::GreaterEqual,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                });
        
        Self {
            pipeline,
            uniform_bind_group_layout,
        }
    }

    pub fn add_to_graph<'node>(
        &'node mut self,
        graph: &mut rend3::graph::RenderGraph<'node>,
        depth_target: rend3::graph::RenderTargetHandle,
        output: rend3::graph::RenderTargetHandle,
    ) {
        let grid_uniform_bg = graph.add_data::<wgpu::BindGroup>();
        self.create_bind_groups(graph, grid_uniform_bg);
        self.render(graph, depth_target, output, grid_uniform_bg);
    }

    /// Adds a node to the render graph which is responsible for filling the grid uniform bind
    /// group resource. To do this, it reconstructs the grid uniform using the view/projection
    /// matrices from the camera manager attached to the graph.
    fn create_bind_groups<'node>(
        &'node self,
        graph: &mut rend3::graph::RenderGraph<'node>,
        grid_uniform_bg: rend3::graph::DataHandle<wgpu::BindGroup>,
    ) {
        let mut builder = graph.add_node("build grid uniforms");

        let output_handle = builder.add_data(grid_uniform_bg, rend3::graph::NodeResourceUsage::Output);
        builder.build(
            move |ctx| {
                let uniform = GridUniform::new(&ctx.data_core.camera_manager);
                let uniform_buffer = ctx.renderer 
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("grid buffer"),
                        contents: bytemuck::cast_slice(&[uniform]),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                let uniform_bind_group = ctx.renderer 
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &self.uniform_bind_group_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.as_entire_binding(),
                        }],
                        label: Some("grid bind group"),
                    });

                ctx.graph_data.set_data(output_handle, Some(uniform_bind_group));
            },
        );
    }

    /// Adds a node to the render graph which is responsible for rendering the grid. This is where
    /// the actual draw calls are given. This node assumes the bind group resource has already been
    /// filled by `create_bind_groups()`.
    pub fn render<'node>(
        &'node self,
        graph: &mut rend3::graph::RenderGraph<'node>,
        depth_target: rend3::graph::RenderTargetHandle,
        output: rend3::graph::RenderTargetHandle,
        uniform_bind_group: rend3::graph::DataHandle<wgpu::BindGroup>,
    ) {
        let mut builder = graph.add_node("grid");

        let depth_handle = builder.add_render_target(depth_target, rend3::graph::NodeResourceUsage::InputOutput);
        let output_handle = builder.add_render_target(output, rend3::graph::NodeResourceUsage::InputOutput);

        let rpass_handle = builder.add_renderpass(rend3::graph::RenderPassTargets {
            targets: vec![rend3::graph::RenderPassTarget {
                color: output_handle,
                clear: rend3::types::Color::BLACK,
                resolve: None, // TODO
            }],
            depth_stencil: Some(rend3::graph::RenderPassDepthTarget {
                target: depth_handle,
                depth_clear: Some(0.),
                stencil_clear: None,
            }),
        });

        let uniform_handle = builder.add_data(uniform_bind_group, rend3::graph::NodeResourceUsage::Input);

        builder.build(move |mut ctx| {
            let rpass = ctx.encoder_or_pass.take_rpass(rpass_handle);
            let grid_uniform_bg = ctx.graph_data.get_data(ctx.temps, uniform_handle).unwrap();

            rpass.set_bind_group(0, grid_uniform_bg, &[]);
            rpass.set_pipeline(&self.pipeline);
            rpass.draw(0..6, 0..1);
        });
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridUniform {
    // projection_matrix * view_matrix (column vectors)
    view_projection_matrix: [[f32; 4]; 4],
    // = view_inv * proj_inv 
    view_projection_matrix_inverse: [[f32; 4]; 4],
    z_near: f32,
    z_far: f32,
    // Warning: The alignment is sizeof([f32; 4]) = 16, but this does NOT mean that each f32
    // needs 12 bytes of padding. It seems them next to each other, as long as we end on the
    // right alignment size. For example, here we need 8 bytes of padding, but if we had a
    // single f32 followed by a vec4<f32>, then the f32 would need 12 bytes of padding.
    _padding: [i32; 2],
}

impl GridUniform {
    /// - `camera`: the rend3 camera from which we get the view-projection matrix and the near
    /// plane.  TODO
    /// - `z_far`: the far plane, since rend3 assumes an infinite far plane.
    pub fn new(camera_manager: &rend3::managers::CameraManager) -> Self {
        let view_projection_matrix = camera_manager.view_proj();
        let view_projection_matrix_inverse = view_projection_matrix.inverse();

        Self {
            view_projection_matrix: *view_projection_matrix.as_ref_parts(),
            view_projection_matrix_inverse: *view_projection_matrix_inverse.as_ref_parts(),
            z_near: 0.001, // TODO
            z_far: 100.,
            _padding: [0; 2],
        }
    }

    pub fn update_matrix(&mut self) {
        todo!()
    }
}

