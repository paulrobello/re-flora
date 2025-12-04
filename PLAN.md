# Coding Plans

## Erase special feature and extension usage

Right now this project will crash on MacOS with M4 Pro chip, with the following error message:

```plaintext
--- Device capability check failed for "Apple M4 Pro" ---
+-----------+----------------------------+----------------------------------------------------------------------------------------------------------------------------------------------------+
| Type      | Name                       | Details                                                                                                                                            |
+=============================================================================================================================================================================================+
| Extension | VK_KHR_shader_clock        | Used for time queries and GPU profiling in compute shaders (not reported by the selected physical device)                                          |
|-----------+----------------------------+----------------------------------------------------------------------------------------------------------------------------------------------------|
| Feature   | VK_EXT_shader_atomic_float | Missing capabilities: shader_image_float32_atomics, shader_image_float32_atomic_add, sparse_image_float32_atomics, sparse_image_float32_atomic_add |
|-----------+----------------------------+----------------------------------------------------------------------------------------------------------------------------------------------------|
| Feature   | shader_subgroup_clock      | VK_KHR_shader_clock feature required for GPU timing                                                                                                |
+-----------+----------------------------+----------------------------------------------------------------------------------------------------------------------------------------------------+

thread 'main' panicked at src/vkn/context/device.rs:365:5:
Selected GPU "Apple M4 Pro" lacks required Vulkan capabilities. Please choose a device that provides the extensions/features listed above.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'main' panicked at library/core/src/panicking.rs:225:5:
panic in a function that cannot unwind
stack backtrace:
   0:        0x10128cf24 - std::backtrace_rs::backtrace::libunwind::trace::h72f4b72e0962905d
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/../../backtrace/src/backtrace/libunwind.rs:117:9
   1:        0x10128cf24 - std::backtrace_rs::backtrace::trace_unsynchronized::hff394536698b6b10
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/../../backtrace/src/backtrace/mod.rs:66:14
   2:        0x10128cf24 - std::sys::backtrace::_print_fmt::h64d1e3035850353e
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/sys/backtrace.rs:66:9
   3:        0x10128cf24 - <std::sys::backtrace::BacktraceLock::print::DisplayBacktrace as core::fmt::Display>::fmt::hf35f9734f9a29483
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/sys/backtrace.rs:39:26
   4:        0x1012aa760 - core::fmt::rt::Argument::fmt::hedf6f2a66f855f69
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/fmt/rt.rs:173:76
   5:        0x1012aa760 - core::fmt::write::h60ec6633daab7b35
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/fmt/mod.rs:1468:25
   6:        0x10128a8b0 - std::io::default_write_fmt::h0e30d7b1295222cb
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/io/mod.rs:639:11
   7:        0x10128a8b0 - std::io::Write::write_fmt::hc29709fdab2e34e2
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/io/mod.rs:1954:13
   8:        0x10128cdd8 - std::sys::backtrace::BacktraceLock::print::hca95bffd78053951
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/sys/backtrace.rs:42:9
   9:        0x10128e140 - std::panicking::default_hook::{{closure}}::h357ed4fbef22679d
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/panicking.rs:300:27
  10:        0x10128df98 - std::panicking::default_hook::h0a4e133b151d5758
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/panicking.rs:327:9
  11:        0x10128ebe0 - std::panicking::rust_panic_with_hook::h557a23724a5de839
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/panicking.rs:833:13
  12:        0x10128e7d4 - std::panicking::begin_panic_handler::{{closure}}::h269cace6208fef05
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/panicking.rs:699:13
  13:        0x10128d3d4 - std::sys::backtrace::__rust_end_short_backtrace::h5be0da278f3aaec7
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/sys/backtrace.rs:174:18
  14:        0x10128e4d8 - __rustc[de2ca18b4c54d5b8]::rust_begin_unwind
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/std/src/panicking.rs:697:5
  15:        0x1012ed6ec - core::panicking::panic_nounwind_fmt::runtime::h5c6a5149472cea01
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/panicking.rs:117:22
  16:        0x1012ed6ec - core::panicking::panic_nounwind_fmt::h9825e2aa83719df7
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/intrinsics/mod.rs:2367:9
  17:        0x1012ed764 - core::panicking::panic_nounwind::h4cc28a4411926d9d
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/panicking.rs:225:5
  18:        0x1012ed918 - core::panicking::panic_cannot_unwind::ha4e3ecab6cb0371c
                               at /rustc/1159e78c4747b02ef996e55082b704c09b970588/library/core/src/panicking.rs:346:5
  19:        0x1011d6c58 - winit::platform_impl::macos::app_state::ApplicationDelegate::app_did_finish_launching::h2c0621f16e1c41a1
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
[1]    9278 abort      cargo run
```

The goal is to avoid using these unsupported extensions. These extensions and features are used in the contree builder.

1. Analyze the current steps for creating the contree, and where these unsupported extensions are used

   Write down to this place

2. Analyze how to avoid using these extensions, regarding the context we are using it, and analyze is it possible not to touch the performance of our builder.

   Write down to this place

## Particle System (need testing)

currently, we are passing Vec3 for each partical position to GPU, but we are clamping its position anyway inside the partical.vert shader, so my thoughts are, to only pass integer
  position into the GPU, for lowering the bandwidth consumption. see in_instance_pos for more reference, we are using uvec3 position there. we can utilize the same thing too for our
  particle system.
