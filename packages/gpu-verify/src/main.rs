//! Execute the WGSL this crate emits, on a real Vulkan device, and compare the numbers with the
//! CPU evaluation lane.
//!
//! The repository's WGSL gate (`cargo test -p himari-check --features wgsl-compile`) proves the
//! emitted kernel *compiles*. It has never proved the kernel *computes the right numbers*: naga
//! validates syntax and types, not arithmetic, and no GPU run existed. This binary closes that gap
//! for the shapes it covers:
//!
//! 1. build a formula as a `himari::Expr`;
//! 2. emit the kernel with the production emitter (`himari::wgsl::emit_wgsl_with_params`);
//! 3. wrap that function into a compute pipeline entry point (storage buffers in/out);
//! 4. translate the module to SPIR-V with naga and execute it through Vulkan;
//! 5. evaluate the same formula on the CPU lane for the same inputs and compare.
//!
//! The comparison is against `himari::eval`, i.e. the f64 lane, while the kernel is f32: the
//! tolerance below is the *f32* tolerance, and every result must also be finite.
//!
//! A **software** Vulkan implementation (Mesa lavapipe) is a legitimate device here — it is a
//! real driver path executing real SPIR-V, which is what catches wrong indexing, wrong constant
//! folding and wrong operator precedence. What it cannot prove is hardware-specific behaviour
//! (real-GPU f32 rounding modes, workgroup scheduling, driver bugs), and this binary says so in
//! its report rather than implying more than it measured.

use ash::vk;

/// One verification case: a formula, its parameters, and the input points to evaluate at.
struct Case {
    name: &'static str,
    /// The emitted kernel takes these parameters in order.
    params: &'static [&'static str],
    expr: himari::Expr,
    /// `points[i][k]` is the value of `params[k]` at point `i`.
    points: Vec<Vec<f64>>,
}

fn cases() -> Vec<Case> {
    use himari::{add, div, mul, neg, num, pow, var};
    // Inputs stay in a range where the f32 and f64 lanes must agree well inside the tolerance,
    // and `rho`/`v` are non-negative so `sqrt` and the powers stay in the real lane.
    let pts = |a: &[f64], b: &[f64]| -> Vec<Vec<f64>> {
        a.iter().zip(b.iter()).map(|(x, y)| vec![*x, *y]).collect()
    };
    vec![
        Case {
            name: "kinetic_energy",
            params: &["m", "v"],
            expr: mul(mul(num(0.5), var("m")), pow(var("v"), num(2.0))),
            points: pts(&[1.0, 2.5, 740.0, 0.125], &[2.0, 3.0, 0.5, 11.0]),
        },
        Case {
            name: "horner_poly",
            params: &["x", "k"],
            expr: add(
                add(
                    add(
                        mul(num(3.0), pow(var("x"), num(3.0))),
                        mul(neg(num(2.0)), pow(var("x"), num(2.0))),
                    ),
                    mul(var("k"), var("x")),
                ),
                num(7.0),
            ),
            points: pts(&[0.5, -1.25, 3.0, 0.0], &[1.0, 2.0, -4.0, 9.5]),
        },
        Case {
            name: "sqrt_and_division",
            params: &["a", "b"],
            expr: div(
                himari::Expr::Call(
                    "sqrt".into(),
                    vec![add(pow(var("a"), num(2.0)), pow(var("b"), num(2.0)))],
                ),
                add(num(1.0), var("a")),
            ),
            points: pts(&[3.0, 0.5, 12.0, 1.0], &[4.0, 0.25, 5.0, 1.0]),
        },
        Case {
            name: "transcendental",
            params: &["t", "w"],
            expr: add(
                mul(
                    var("t"),
                    himari::Expr::Call("exp".into(), vec![neg(div(var("t"), num(4.0)))]),
                ),
                himari::Expr::Call("sin".into(), vec![mul(var("w"), var("t"))]),
            ),
            points: pts(&[0.0, 1.0, 2.5, 4.0], &[0.5, 1.5, -1.0, 3.25]),
        },
    ]
}

/// The compute entry point that drives one emitted kernel.
///
/// Parameters arrive interleaved (`params[i * n + k]`) in one read-only storage buffer and the
/// results leave through a second one, so the binding layout is identical for every case and the
/// shader contains no per-case bookkeeping that could mask an emitter bug.
fn entry_point(kernel_name: &str, params: &[&str], points: usize) -> String {
    let args: Vec<String> = params
        .iter()
        .enumerate()
        .map(|(k, _)| format!("input.data[i * {n} + {k}]", n = params.len(), k = k))
        .collect();
    format!(
        "\nstruct Buffer {{ data: array<f32> }};\n\
         @group(0) @binding(0) var<storage, read> input: Buffer;\n\
         @group(0) @binding(1) var<storage, read_write> output: Buffer;\n\
         @compute @workgroup_size(64)\n\
         fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{\n\
         \x20   let i = gid.x;\n\
         \x20   if (i >= {points}u) {{ return; }}\n\
         \x20   output.data[i] = {kernel_name}({args});\n\
         }}\n",
        points = points,
        kernel_name = kernel_name,
        args = args.join(", ")
    )
}

/// WGSL → SPIR-V. `naga` refuses anything the driver would refuse, which is why the harness
/// compiles rather than trusting the emitter's own static screen.
fn to_spirv(wgsl: &str) -> Result<Vec<u32>, String> {
    let module = naga::front::wgsl::parse_str(wgsl)
        .map_err(|e| format!("WGSL parse failed:\n{}", e.emit_to_string(wgsl)))?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|e| format!("WGSL validation failed: {e:?}"))?;
    let opts = naga::back::spv::Options::default();
    naga::back::spv::write_vec(&module, &info, &opts, None)
        .map_err(|e| format!("SPIR-V backend failed: {e:?}"))
}

/// A Vulkan compute device, kept alive for the whole run.
struct Gpu {
    _entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    queue: vk::Queue,
    queue_family: u32,
    memory_type: u32,
    name: String,
}

impl Gpu {
    fn new() -> Result<Self, String> {
        unsafe {
            let entry = ash::Entry::load().map_err(|e| format!("no Vulkan loader: {e}"))?;
            let app = vk::ApplicationInfo::default()
                .application_name(c"himari-gpu-verify")
                .api_version(vk::make_api_version(0, 1, 1, 0));
            let instance = entry
                .create_instance(
                    &vk::InstanceCreateInfo::default().application_info(&app),
                    None,
                )
                .map_err(|e| format!("vkCreateInstance failed: {e}"))?;
            let physicals = instance
                .enumerate_physical_devices()
                .map_err(|e| format!("vkEnumeratePhysicalDevices failed: {e}"))?;
            let mut chosen = None;
            for p in physicals {
                let fams = instance.get_physical_device_queue_family_properties(p);
                if let Some(fam) = fams
                    .iter()
                    .position(|q| q.queue_flags.contains(vk::QueueFlags::COMPUTE))
                {
                    chosen = Some((p, fam as u32));
                    break;
                }
            }
            let (physical, queue_family) =
                chosen.ok_or("no Vulkan device with a compute queue (install Mesa lavapipe)")?;
            let props = instance.get_physical_device_properties(physical);
            let name = props
                .device_name_as_c_str()
                .unwrap_or(c"unknown")
                .to_string_lossy()
                .into_owned();
            let priority = [1.0f32];
            let queue_info = [vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family)
                .queue_priorities(&priority)];
            let device = instance
                .create_device(
                    physical,
                    &vk::DeviceCreateInfo::default().queue_create_infos(&queue_info),
                    None,
                )
                .map_err(|e| format!("vkCreateDevice failed: {e}"))?;
            let queue = device.get_device_queue(queue_family, 0);
            // Host-visible + host-coherent memory keeps the harness free of staging copies; it is
            // slower, and speed is not what is being measured here.
            let mem_props = instance.get_physical_device_memory_properties(physical);
            let memory_type = (0..mem_props.memory_type_count)
                .find(|&i| {
                    let flags = mem_props.memory_types[i as usize].property_flags;
                    flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE)
                        && flags.contains(vk::MemoryPropertyFlags::HOST_COHERENT)
                })
                .ok_or("no host-visible coherent memory type")?;
            Ok(Self {
                _entry: entry,
                instance,
                device,
                queue,
                queue_family,
                memory_type,
                name,
            })
        }
    }

    /// Run one kernel over `points` points and return the results.
    fn run(&self, spirv: &[u32], input: &[f32], points: usize) -> Result<Vec<f32>, String> {
        unsafe {
            let module = self
                .device
                .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(spirv), None)
                .map_err(|e| format!("shader module: {e}"))?;
            let bindings = [
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE),
            ];
            let set_layout = self
                .device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                    None,
                )
                .map_err(|e| format!("descriptor set layout: {e}"))?;
            let set_layouts = [set_layout];
            let pipeline_layout = self
                .device
                .create_pipeline_layout(
                    &vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts),
                    None,
                )
                .map_err(|e| format!("pipeline layout: {e}"))?;
            let stage = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(module)
                .name(c"main");
            let pipeline = self
                .device
                .create_compute_pipelines(
                    vk::PipelineCache::null(),
                    &[vk::ComputePipelineCreateInfo::default()
                        .stage(stage)
                        .layout(pipeline_layout)],
                    None,
                )
                .map_err(|(_, e)| format!("compute pipeline: {e}"))?[0];

            let in_bytes = (input.len() * 4) as u64;
            let out_bytes = (points * 4) as u64;
            let in_buf = self.make_buffer(in_bytes)?;
            let out_buf = self.make_buffer(out_bytes)?;
            let (in_mem, in_ptr) = self.alloc_bind(in_buf, in_bytes)?;
            let (out_mem, out_ptr) = self.alloc_bind(out_buf, out_bytes)?;
            std::ptr::copy_nonoverlapping(input.as_ptr() as *const u8, in_ptr, input.len() * 4);
            std::ptr::write_bytes(out_ptr, 0, points * 4);

            let pool_sizes = [vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(2)];
            let pool = self
                .device
                .create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::default()
                        .pool_sizes(&pool_sizes)
                        .max_sets(1),
                    None,
                )
                .map_err(|e| format!("descriptor pool: {e}"))?;
            let set = self
                .device
                .allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(pool)
                        .set_layouts(&set_layouts),
                )
                .map_err(|e| format!("descriptor set: {e}"))?[0];
            let in_info = [vk::DescriptorBufferInfo::default()
                .buffer(in_buf)
                .offset(0)
                .range(vk::WHOLE_SIZE)];
            let out_info = [vk::DescriptorBufferInfo::default()
                .buffer(out_buf)
                .offset(0)
                .range(vk::WHOLE_SIZE)];
            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&in_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&out_info),
            ];
            self.device.update_descriptor_sets(&writes, &[]);

            let cmd_pool = self
                .device
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::default().queue_family_index(self.queue_family),
                    None,
                )
                .map_err(|e| format!("command pool: {e}"))?;
            let cmd = self
                .device
                .allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::default()
                        .command_pool(cmd_pool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1),
                )
                .map_err(|e| format!("command buffer: {e}"))?[0];
            self.device
                .begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default())
                .map_err(|e| format!("begin: {e}"))?;
            self.device
                .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, pipeline);
            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                pipeline_layout,
                0,
                &[set],
                &[],
            );
            let groups = (points as u64).div_ceil(64);
            self.device.cmd_dispatch(cmd, groups as u32, 1, 1);
            // A storage-buffer write followed by a host read needs both barriers: the first makes
            // the shader writes visible, the second makes them visible to the host.
            let sh_barrier = [vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::HOST_READ)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(out_buf)
                .offset(0)
                .size(vk::WHOLE_SIZE)];
            self.device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &[],
                &sh_barrier,
                &[],
            );
            self.device
                .end_command_buffer(cmd)
                .map_err(|e| format!("end: {e}"))?;
            let cmds = [cmd];
            let submit = [vk::SubmitInfo::default().command_buffers(&cmds)];
            let fence = self
                .device
                .create_fence(&vk::FenceCreateInfo::default(), None)
                .map_err(|e| format!("fence: {e}"))?;
            self.device
                .queue_submit(self.queue, &submit, fence)
                .map_err(|e| format!("submit: {e}"))?;
            self.device
                .wait_for_fences(&[fence], true, 60_000_000_000)
                .map_err(|e| format!("the dispatch did not finish: {e}"))?;

            let mut out = vec![0f32; points];
            std::ptr::copy_nonoverlapping(
                out_ptr as *const u8,
                out.as_mut_ptr() as *mut u8,
                points * 4,
            );

            self.device.destroy_fence(fence, None);
            self.device.destroy_command_pool(cmd_pool, None);
            self.device.destroy_descriptor_pool(pool, None);
            self.device.unmap_memory(in_mem);
            self.device.unmap_memory(out_mem);
            self.device.destroy_buffer(in_buf, None);
            self.device.destroy_buffer(out_buf, None);
            self.device.free_memory(in_mem, None);
            self.device.free_memory(out_mem, None);
            self.device.destroy_pipeline(pipeline, None);
            self.device.destroy_pipeline_layout(pipeline_layout, None);
            self.device.destroy_descriptor_set_layout(set_layout, None);
            self.device.destroy_shader_module(module, None);
            Ok(out)
        }
    }

    unsafe fn make_buffer(&self, bytes: u64) -> Result<vk::Buffer, String> {
        self.device
            .create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(bytes)
                    .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )
            .map_err(|e| format!("buffer: {e}"))
    }

    unsafe fn alloc_bind(
        &self,
        buffer: vk::Buffer,
        bytes: u64,
    ) -> Result<(vk::DeviceMemory, *mut u8), String> {
        let req = self.device.get_buffer_memory_requirements(buffer);
        let mem = self
            .device
            .allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(req.size.max(bytes))
                    .memory_type_index(self.memory_type),
                None,
            )
            .map_err(|e| format!("memory: {e}"))?;
        self.device
            .bind_buffer_memory(buffer, mem, 0)
            .map_err(|e| format!("bind: {e}"))?;
        let ptr = self
            .device
            .map_memory(mem, 0, req.size.max(bytes), vk::MemoryMapFlags::empty())
            .map_err(|e| format!("map: {e}"))? as *mut u8;
        Ok((mem, ptr))
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

fn main() -> std::process::ExitCode {
    let gpu = match Gpu::new() {
        Ok(g) => g,
        Err(why) => {
            eprintln!("gpu-verify: cannot run: {why}");
            eprintln!("gpu-verify: NOTHING WAS VERIFIED (this exit code does not mean the kernels are wrong)");
            return std::process::ExitCode::from(2);
        }
    };
    println!("gpu-verify: device = {}", gpu.name);
    println!(
        "gpu-verify: this is a Vulkan implementation executing real SPIR-V; it does not measure \
         hardware-specific behaviour (real-GPU rounding, workgroup scheduling, driver bugs)"
    );

    let mut failures = 0usize;
    let mut checks = 0usize;
    for case in cases() {
        let kernel = match himari::wgsl::emit_wgsl_with_params(
            &case.expr,
            case.name,
            &case
                .params
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>(),
        ) {
            Ok(k) => k,
            Err(e) => {
                println!("  {}: EMIT FAILED — {e}", case.name);
                failures += 1;
                continue;
            }
        };
        let module = format!(
            "{kernel}{}",
            entry_point(case.name, case.params, case.points.len())
        );
        let spirv = match to_spirv(&module) {
            Ok(s) => s,
            Err(e) => {
                println!("  {}: COMPILE FAILED — {e}", case.name);
                failures += 1;
                continue;
            }
        };
        let mut flat = Vec::with_capacity(case.points.len() * case.params.len());
        for p in &case.points {
            flat.extend(p.iter().map(|v| *v as f32));
        }
        let got = match gpu.run(&spirv, &flat, case.points.len()) {
            Ok(v) => v,
            Err(e) => {
                println!("  {}: DISPATCH FAILED — {e}", case.name);
                failures += 1;
                continue;
            }
        };

        // The CPU reference: the same formula through the f64 evaluation lane.
        let mut worst_rel = 0.0f64;
        let mut bad = 0usize;
        for (i, point) in case.points.iter().enumerate() {
            let env: std::collections::HashMap<String, f64> = case
                .params
                .iter()
                .zip(point.iter())
                .map(|(name, value)| (name.to_string(), *value))
                .collect();
            let want = match himari::eval(&case.expr, &env) {
                Ok(v) => v,
                Err(e) => {
                    println!("  {} point {i}: the CPU lane refused it — {e}", case.name);
                    failures += 1;
                    continue;
                }
            };
            let actual = got[i] as f64;
            if !actual.is_finite() {
                println!("  {} point {i}: the kernel produced {actual}", case.name);
                bad += 1;
                continue;
            }
            let rel = if want.abs() > 1e-12 {
                (actual - want).abs() / want.abs()
            } else {
                (actual - want).abs()
            };
            worst_rel = worst_rel.max(rel);
            // f32 has ~7 decimal digits; a transcendental chain over a handful of operations
            // lands well inside 1e-5 relative, and anything above that means a real divergence
            // (wrong operator, wrong constant, wrong index) rather than rounding.
            if rel > 1e-5 {
                println!(
                    "  {} point {i}: kernel {actual} vs CPU {want} (relative {rel:e})",
                    case.name
                );
                bad += 1;
            }
            checks += 1;
        }
        if bad == 0 {
            println!(
                "  {}: {} points ok (worst relative error {worst_rel:.3e})",
                case.name,
                case.points.len()
            );
        } else {
            failures += 1;
        }
    }

    if failures == 0 {
        println!("gpu-verify: PASS — {checks} points agreed with the CPU lane");
        std::process::ExitCode::SUCCESS
    } else {
        println!("gpu-verify: FAIL — {failures} case(s) diverged");
        std::process::ExitCode::FAILURE
    }
}
