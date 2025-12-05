# Coding Plans

## Erase special feature and extension usage

Right now this project will crash on MacOS with M4 Pro chip, with the following error message:

```plaintext
[16:15:24.617 INFO re_flora::app::core] sum: 3
Failed to grab cursor: NotSupported(NotSupportedError)
--- Physical Device Evaluation Report ---
+--------------+----------------+-------------+-------------+-----------------------+
| Device       | Type           | Memory (MB) | Suitability | Reason                |
+===================================================================================+
| Apple M4 Pro | INTEGRATED_GPU | 49152.00    | Suitable    | All requirements met. |
+--------------+----------------+-------------+-------------+-----------------------+

--- Suitable Physical Devices ---
+--------------+----------------+-------------+-------+-----------+
| Device       | Type           | Memory (MB) | Score | Selected? |
+=================================================================+
| Apple M4 Pro | INTEGRATED_GPU | 49152.00    | 242   | Yes       |
+--------------+----------------+-------------+-------+-----------+

--- Queue Family Analysis for Selected Device ---
+--------------------+----------+---------+---------+----------+----------------+
| Queue Family Index | Graphics | Present | Compute | Transfer | Sparse Binding |
+===============================================================================+
| 0                  | Yes      | Yes     | Yes     | Yes      |                |
|--------------------+----------+---------+---------+----------+----------------|
| 1                  | Yes      | Yes     | Yes     | Yes      |                |
|--------------------+----------+---------+---------+----------+----------------|
| 2                  | Yes      | Yes     | Yes     | Yes      |                |
|--------------------+----------+---------+---------+----------+----------------|
| 3                  | Yes      | Yes     | Yes     | Yes      |                |
+--------------------+----------+---------+---------+----------+----------------+

--- Selected Queue Family Indices ---
+--------------------------------------+--------------------+
| Queue Type                           | Queue Family Index |
+===========================================================+
| General (Graphics, Present, Compute) | 0                  |
|--------------------------------------+--------------------|
| Dedicated Transfer (if available)    | 1                  |
+--------------------------------------+--------------------+
[16:15:24.652 INFO re_flora::vkn::context::physical_device] Selected physical device: Apple M4 Pro

--- Device capability check failed for "Apple M4 Pro" ---
+---------+----------------------------+----------------------------------------------------------------------------------------------------------------------------------------------------+
| Type    | Name                       | Details                                                                                                                                            |
+===========================================================================================================================================================================================+
| Feature | VK_EXT_shader_atomic_float | Missing capabilities: shader_image_float32_atomics, shader_image_float32_atomic_add, sparse_image_float32_atomics, sparse_image_float32_atomic_add |
+---------+----------------------------+----------------------------------------------------------------------------------------------------------------------------------------------------+

thread 'main' panicked at src/vkn/context/device.rs:366:5:
Selected GPU "Apple M4 Pro" lacks required Vulkan capabilities. Please choose a device that provides the extensions/features listed above.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'main' panicked at library/core/src/panicking.rs:225:5:
panic in a function that cannot unwind
stack backtrace:
   0:        0x10567e6e0 - std::backtrace_rs::backtrace::libunwind::trace::h72f4b72e0962905d
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/../../backtrace/src/backtrace/libunwind.rs:117:9
   1:        0x10567e6e0 - std::backtrace_rs::backtrace::trace_unsynchronized::hff394536698b6b10
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/../../backtrace/src/backtrace/mod.rs:66:14
   2:        0x10567e6e0 - std::sys::backtrace::_print_fmt::h64d1e3035850353e
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/sys/backtrace.rs:66:9
   3:        0x10567e6e0 - <std::sys::backtrace::BacktraceLock::print::DisplayBacktrace as core::fmt::Display>::fmt::hf35f9734f9a29483
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/sys/backtrace.rs:39:26
   4:        0x10569bf1c - core::fmt::rt::Argument::fmt::hedf6f2a66f855f69
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/fmt/rt.rs:173:76
   5:        0x10569bf1c - core::fmt::write::h60ec6633daab7b35
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/fmt/mod.rs:1468:25
   6:        0x10567c06c - std::io::default_write_fmt::h0e30d7b1295222cb
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/io/mod.rs:639:11
   7:        0x10567c06c - std::io::Write::write_fmt::hc29709fdab2e34e2
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/io/mod.rs:1954:13
   8:        0x10567e594 - std::sys::backtrace::BacktraceLock::print::hca95bffd78053951
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/sys/backtrace.rs:42:9
   9:        0x10567f8fc - std::panicking::default_hook::{{closure}}::h357ed4fbef22679d
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/panicking.rs:300:27
  10:        0x10567f754 - std::panicking::default_hook::h0a4e133b151d5758
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/panicking.rs:327:9
  11:        0x10568039c - std::panicking::rust_panic_with_hook::h557a23724a5de839
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/panicking.rs:833:13
  12:        0x10567ff90 - std::panicking::begin_panic_handler::{{closure}}::h269cace6208fef05
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/panicking.rs:699:13
  13:        0x10567eb90 - std::sys::backtrace::__rust_end_short_backtrace::h5be0da278f3aaec7
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/sys/backtrace.rs:174:18
  14:        0x10567fc94 - __rustc[de2ca18b4c54d5b8]::rust_begin_unwind
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/panicking.rs:697:5
  15:        0x1056df8bc - core::panicking::panic_nounwind_fmt::runtime::h5c6a5149472cea01
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/panicking.rs:117:22
  16:        0x1056df8bc - core::panicking::panic_nounwind_fmt::h9825e2aa83719df7
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/intrinsics/mod.rs:2367:9
  17:        0x1056df934 - core::panicking::panic_nounwind::h4cc28a4411926d9d
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/panicking.rs:225:5
  18:        0x1056dfae8 - core::panicking::panic_cannot_unwind::ha4e3ecab6cb0371c
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/panicking.rs:346:5
  19:        0x1055c8414 - winit::platform_impl::macos::app_state::ApplicationDelegate::app_did_finish_launching::h2c0621f16e1c41a1
                               at /Users/bytedance/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.5.2/src/macros/declare_class.rs:981:25
  20:        0x19718b46c - <unknown>
  21:        0x19721ab28 - <unknown>
  22:        0x19721aa6c - <unknown>
  23:        0x19715a8b8 - <unknown>
  24:        0x198714680 - <unknown>
  25:        0x19b0c21bc - <unknown>
  26:        0x19b0c1f6c - <unknown>
  27:        0x19b0c0568 - <unknown>
  28:        0x19b0c017c - <unknown>
  29:        0x19873ce40 - <unknown>
  30:        0x19873cc38 - <unknown>
thread caused non-unwinding panic. aborting.
[1]    72000 abort      cargo run
(base) ➜  re-flora git:(dev) ✗ 
  29:        0x19873ce40 - <unknown>
  30:        0x19873cc38 - <unknown>
thread caused non-unwinding panic. aborting.
```

The goal is to avoid using these unsupported extensions. These extensions and features are used in the contree builder.

1. Analyze the current steps for creating the contree, and where these unsupported extensions are used

   The command buffer recorded in `src/builder/contree/mod.rs:190-299` executes the builder as a series of compute passes:
   - `buffer_setup.comp` initializes `ContreeBuildState`, zeroes the per-level counters, and writes the indirect dispatch sizes for later passes.
   - `leaf_write.comp` scans each 4×4×4 brick of the source surface, builds leaf payloads, and uses `atomicAdd` on `contree_build_result.leaf_len` to reserve slots (`shader/builder/contree/leaf_write.comp:62`). The atomics are purely 32-bit integer SSBO operations.
   - `buffer_update.comp` shrinks the working dimension by 4× in each axis and recomputes the indirect dispatch arguments for the next tree level.
   - `tree_write.comp` performs the higher-level reductions. Each invocation reads the child bricks, determines which of the 64 children exist, and writes them to the dense buffer. It maintains the per-level prefix offsets with `atomicAdd` on `counter_for_levels` and `contree_build_result.node_len` (`shader/builder/contree/tree_write.comp:56-58`).
   - `last_buffer_update.comp` copies the root node into the dense buffer, fixes up the node counters, and emits the indirect dispatch arguments for the final concatenation.
   - `concat.comp` walks the dense per-level chunks and writes the final packed tree into `contree_node_data`.

   Within these passes the only extensions pulled in are `GL_GOOGLE_include_directive` (for `#include`) and `GL_ARB_gpu_shader_int64` (to express the 64-bit child masks in `shader/include/contree_node.glsl`). There are no references to `GL_ARB_shader_clock` or any of the `GL_EXT_shader_atomic_float` capabilities in the contree shaders. GPU timing helpers live in `shader/include/core/shader_clock.glsl` and are only included from `shader/tracer/tracer.comp:101`, while every `atomicAdd` the contree builder performs operates on unsigned integers in SSBOs. The crash therefore stems from `src/vkn/context/device.rs:127-317` unconditionally requesting `VK_KHR_shader_clock` and `VK_EXT_shader_atomic_float`, not from the builder code actually relying on them.

2. Analyze how to avoid using these extensions, regarding the context we are using it, and analyze is it possible not to touch the performance of our builder.

   Because the contree shaders already stick to integer atomics and never read a shader clock, we can stop requesting the unsupported capabilities without touching the builder’s data flow:
   - Remove `VK_KHR_shader_clock` from `device_extension_requirements()` and from the `vk::DeviceCreateInfo` `pNext` chain (`src/vkn/context/device.rs`). If GPU-side profiling is still desirable on desktop GPUs, gate the timing helper behind a feature flag (e.g., define `ENABLE_SHADER_CLOCK`) so the shader include and extension pragma in `shader/include/core/shader_clock.glsl` are only compiled when explicitly enabled. For routine builds (macOS/MoltenVK) we fall back to regular `vkCmdWriteTimestamp` calls between dispatches for timing, which keeps the contree command buffer identical and has zero shader cost.
   - Drop the unconditional `VK_EXT_shader_atomic_float` requirement and the corresponding `vk::PhysicalDeviceShaderAtomicFloatFeaturesEXT` block. The contree builder’s counters (`contree_build_result`, `counter_for_levels`) are all `uint`, and even the surface sampling path uses an `r32ui` image (`shader/builder/contree/leaf_write.comp:32`). As long as we keep the buffers in integer formats (which they already are), removing the feature flag does not alter the kernels or their occupancy. The memory-layout (shared memory scans, prefix sums, indirect dispatch reuse) stays the same, so performance remains unchanged.
   - After the device stops requesting those features by default, add an opt-in path (e.g., a debug CLI flag) that appends them back in for developers who want shader-clock profiling on compatible hardware. This keeps high-end GPUs debuggable while letting Apple GPUs run the builder without capability failures.

## Particle System (need testing)

currently, we are passing Vec3 for each partical position to GPU, but we are clamping its position anyway inside the partical.vert shader, so my thoughts are, to only pass integer
  position into the GPU, for lowering the bandwidth consumption. see in_instance_pos for more reference, we are using uvec3 position there. we can utilize the same thing too for our
  particle system.
