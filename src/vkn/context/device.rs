use super::Queue;
use super::{instance::Instance, physical_device::PhysicalDevice, queue::QueueFamilyIndices};
use ash::vk;
use comfy_table::Table;
use std::{collections::HashSet, ffi::CStr, fmt::Debug, sync::Arc};

#[derive(Clone)]
struct DeviceExtensionRequirement {
    name: &'static CStr,
    reason: &'static str,
}

struct DeviceInner {
    device: ash::Device,
}

impl Drop for DeviceInner {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device.destroy_device(None);
        }
    }
}

#[derive(Clone)]
pub struct Device(Arc<DeviceInner>);

impl std::ops::Deref for Device {
    type Target = ash::Device;
    fn deref(&self) -> &Self::Target {
        &self.0.device
    }
}

impl Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Device({:?})", self.0.device.handle())
    }
}

impl PartialEq for Device {
    fn eq(&self, other: &Self) -> bool {
        self.0.device.handle() == other.0.device.handle()
    }
}

impl Device {
    pub fn new(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        queue_family_indices: &QueueFamilyIndices,
    ) -> Self {
        let physical_device_raw = physical_device.as_raw();
        let extension_requirements = device_extension_requirements();
        validate_device_capabilities(
            instance.as_raw(),
            physical_device_raw,
            &extension_requirements,
        );
        let device = create_device(
            instance.as_raw(),
            physical_device_raw,
            queue_family_indices,
            &extension_requirements,
        );
        Self(Arc::new(DeviceInner { device }))
    }

    pub fn as_raw(&self) -> &ash::Device {
        &self.0.device
    }

    pub fn wait_queue_idle(&self, queue: &Queue) {
        unsafe { self.as_raw().queue_wait_idle(queue.as_raw()).unwrap() };
    }

    #[allow(unused)]
    pub fn wait_idle(&self) {
        unsafe { self.as_raw().device_wait_idle().unwrap() };
    }

    /// Get a queue from the device, only the first queue is returned in current implementation
    pub fn get_queue(&self, queue_family_index: u32) -> Queue {
        let queue = unsafe { self.as_raw().get_device_queue(queue_family_index, 0) };
        Queue::new(queue)
    }
}

fn create_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_family_indices: &QueueFamilyIndices,
    extension_requirements: &[DeviceExtensionRequirement],
) -> ash::Device {
    let queue_priorities = [1.0f32];
    let queue_create_infos = {
        let mut indices = HashSet::new();
        for idx in queue_family_indices.get_all_indices() {
            indices.insert(idx);
        }
        indices
            .into_iter()
            .map(|index| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(index)
                    .queue_priorities(&queue_priorities)
            })
            .collect::<Vec<_>>()
    };

    let extension_ptrs: Vec<*const i8> = extension_requirements
        .iter()
        .map(|req| req.name.as_ptr())
        .collect();

    let physical_device_features = vk::PhysicalDeviceFeatures {
        shader_int64: vk::TRUE,
        ..Default::default()
    };

    let mut buffer_device_address_features = vk::PhysicalDeviceBufferDeviceAddressFeatures {
        buffer_device_address: vk::TRUE,
        ..Default::default()
    };

    let mut physical_device_shader_atomic_float_features_khr =
        vk::PhysicalDeviceShaderAtomicFloatFeaturesEXT {
            shader_buffer_float32_atomics: vk::TRUE,
            shader_buffer_float32_atomic_add: vk::TRUE,
            shader_shared_float32_atomics: vk::TRUE,
            shader_shared_float32_atomic_add: vk::TRUE,
            shader_image_float32_atomics: vk::TRUE,
            shader_image_float32_atomic_add: vk::TRUE,
            sparse_image_float32_atomics: vk::TRUE,
            sparse_image_float32_atomic_add: vk::TRUE,
            ..Default::default()
        };

    // let mut physical_device_acceleration_structure_features_khr =
    //     vk::PhysicalDeviceAccelerationStructureFeaturesKHR {
    //         acceleration_structure: vk::TRUE,
    //         ..Default::default()
    //     };
    // let mut physical_device_ray_query_features_khr = vk::PhysicalDeviceRayQueryFeaturesKHR {
    //     ray_query: vk::TRUE,
    //     ..Default::default()
    // };

    let mut physical_device_shader_clock_features_khr = vk::PhysicalDeviceShaderClockFeaturesKHR {
        shader_subgroup_clock: vk::TRUE,
        ..Default::default()
    };

    let device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&extension_ptrs)
        .enabled_features(&physical_device_features)
        .push_next(&mut buffer_device_address_features)
        .push_next(&mut physical_device_shader_clock_features_khr)
        .push_next(&mut physical_device_shader_atomic_float_features_khr);

    unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .expect("Failed to create logical device")
    }
}

fn device_extension_requirements() -> Vec<DeviceExtensionRequirement> {
    let mut requirements = vec![
        DeviceExtensionRequirement {
            name: vk::KHR_SWAPCHAIN_NAME,
            reason: "Required to present rendered images to the window surface",
        },
        DeviceExtensionRequirement {
            name: vk::KHR_DEFERRED_HOST_OPERATIONS_NAME,
            reason:
                "Needed for `VK_KHR_acceleration_structure` companion functionality (shader builds)",
        },
        DeviceExtensionRequirement {
            name: vk::KHR_SHADER_CLOCK_NAME,
            reason: "Used for time queries and GPU profiling in compute shaders",
        },
        DeviceExtensionRequirement {
            name: vk::EXT_SHADER_ATOMIC_FLOAT_NAME,
            reason: "Required for float atomics inside compute pipelines",
        },
    ];

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        requirements.push(DeviceExtensionRequirement {
            name: ash::khr::portability_subset::NAME,
            reason: "macOS/iOS MoltenVK portability requirements",
        });
    }

    requirements
}

fn collect_missing_extension_rows(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    requirements: &[DeviceExtensionRequirement],
) -> Vec<(String, String)> {
    let properties = unsafe {
        instance
            .enumerate_device_extension_properties(physical_device)
            .expect("Failed to enumerate device extension properties")
    };

    requirements
        .iter()
        .filter(|req| {
            !properties.iter().any(|ext| {
                let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
                name == req.name
            })
        })
        .map(|req| {
            (
                req.name.to_string_lossy().into_owned(),
                req.reason.to_string(),
            )
        })
        .collect()
}

fn collect_missing_feature_rows(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> Vec<(String, String)> {
    let mut rows = Vec::new();

    let mut buffer_device_address_features =
        vk::PhysicalDeviceBufferDeviceAddressFeatures::default();
    let mut shader_atomic_float_features =
        vk::PhysicalDeviceShaderAtomicFloatFeaturesEXT::default();
    let mut shader_clock_features = vk::PhysicalDeviceShaderClockFeaturesKHR::default();

    let mut features2 = vk::PhysicalDeviceFeatures2::default()
        .push_next(&mut buffer_device_address_features)
        .push_next(&mut shader_clock_features)
        .push_next(&mut shader_atomic_float_features);

    unsafe {
        instance.get_physical_device_features2(physical_device, &mut features2);
    }

    if features2.features.shader_int64 != vk::TRUE {
        rows.push((
            "shaderInt64".to_string(),
            "Core Vulkan feature required for renderer compute passes".to_string(),
        ));
    }

    if buffer_device_address_features.buffer_device_address != vk::TRUE {
        rows.push((
            "bufferDeviceAddress".to_string(),
            "VK_KHR_buffer_device_address feature required for GPU pointers".to_string(),
        ));
    }

    let mut missing_atomic_caps = Vec::new();
    let atomic_requirements = [
        (
            shader_atomic_float_features.shader_buffer_float32_atomics,
            "shader_buffer_float32_atomics",
        ),
        (
            shader_atomic_float_features.shader_buffer_float32_atomic_add,
            "shader_buffer_float32_atomic_add",
        ),
        (
            shader_atomic_float_features.shader_shared_float32_atomics,
            "shader_shared_float32_atomics",
        ),
        (
            shader_atomic_float_features.shader_shared_float32_atomic_add,
            "shader_shared_float32_atomic_add",
        ),
        (
            shader_atomic_float_features.shader_image_float32_atomics,
            "shader_image_float32_atomics",
        ),
        (
            shader_atomic_float_features.shader_image_float32_atomic_add,
            "shader_image_float32_atomic_add",
        ),
        (
            shader_atomic_float_features.sparse_image_float32_atomics,
            "sparse_image_float32_atomics",
        ),
        (
            shader_atomic_float_features.sparse_image_float32_atomic_add,
            "sparse_image_float32_atomic_add",
        ),
    ];

    for (flag, name) in atomic_requirements {
        if flag != vk::TRUE {
            missing_atomic_caps.push(name);
        }
    }

    if !missing_atomic_caps.is_empty() {
        rows.push((
            "VK_EXT_shader_atomic_float".to_string(),
            format!("Missing capabilities: {}", missing_atomic_caps.join(", ")),
        ));
    }

    if shader_clock_features.shader_subgroup_clock != vk::TRUE {
        rows.push((
            "shader_subgroup_clock".to_string(),
            "VK_KHR_shader_clock feature required for GPU timing".to_string(),
        ));
    }

    rows
}

fn validate_device_capabilities(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    extension_requirements: &[DeviceExtensionRequirement],
) {
    let missing_extensions =
        collect_missing_extension_rows(instance, physical_device, extension_requirements);
    let missing_features = collect_missing_feature_rows(instance, physical_device);

    if missing_extensions.is_empty() && missing_features.is_empty() {
        return;
    }

    let props = unsafe { instance.get_physical_device_properties(physical_device) };
    let device_name = unsafe {
        CStr::from_ptr(props.device_name.as_ptr())
            .to_string_lossy()
            .into_owned()
    };

    println!(
        "\n--- Device capability check failed for \"{}\" ---",
        device_name
    );
    let mut table = Table::new();
    table.set_header(vec!["Type", "Name", "Details"]);

    for (ext, detail) in missing_extensions {
        table.add_row(vec![
            "Extension".to_string(),
            ext,
            format!("{detail} (not reported by the selected physical device)"),
        ]);
    }

    for (name, details) in missing_features {
        table.add_row(vec!["Feature".to_string(), name, details]);
    }

    println!("{table}");

    panic!(
        "Selected GPU \"{}\" lacks required Vulkan capabilities. Please choose a device that provides the extensions/features listed above.",
        device_name
    );
}
