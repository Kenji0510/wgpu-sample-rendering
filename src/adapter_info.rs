pub fn display_adapter_limits_info(limits: &wgpu::Limits) {
    println!("Graphics Adapter Limits:");
    println!("  max_bind_groups: {}", limits.max_bind_groups);
    println!(
        "  max_uniform_buffer_binding_size: {} KB",
        limits.max_uniform_buffer_binding_size / 1024
    );
    println!(
        "  max_push_constant_size: {} B",
        limits.max_push_constant_size
    );
    println!("  max_vertex_attributes: {}", limits.max_vertex_attributes);
    println!("  max_vertex_buffers: {}", limits.max_vertex_buffers);

    println!("\nCompute Shader");
    println!(
        "  max_storage_buffer_binding_size: {} MB",
        limits.max_storage_buffer_binding_size / 1024 / 1024
    );
    println!(
        "  max_compute_workgroup_size: x={}, y={}, z={}",
        limits.max_compute_workgroup_size_x,
        limits.max_compute_workgroup_size_y,
        limits.max_compute_workgroup_size_z
    );
    println!(
        "  max_compute_invocations_per_workgroup: {}",
        limits.max_compute_invocations_per_workgroup
    );
    println!(
        "  max_storage_buffers_per_shader_stage: {}",
        limits.max_storage_buffers_per_shader_stage
    );

    println!("\nFragment Shader");
    println!(
        "  max_texture_dimension_2d: {} px",
        limits.max_texture_dimension_2d
    );
    println!(
        "  max_sampled_textures_per_shader_stage: {}",
        limits.max_sampled_textures_per_shader_stage
    );
}
