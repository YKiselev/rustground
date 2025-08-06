use log::warn;
use vulkanalia::{
    Entry,
    loader::{LIBRARY, LibloadingLoader},
};
use winit::window::Window;

use crate::{
    error::{VkError, to_generic},
    instance::VkInstance,
};

#[derive(Debug)]
pub struct VulkanRenderer {
    entry: Entry,
    instance: VkInstance,
}

impl VulkanRenderer {
    pub fn new(window: &Window) -> Result<Self, VkError> {
        unsafe {
            let loader = LibloadingLoader::new(LIBRARY).map_err(to_generic)?;
            let entry = Entry::new(loader).map_err(to_generic)?;
            let instance = VkInstance::new(window, &entry)?;
            Ok(Self { entry, instance })
        }
    }

    pub fn render(&mut self) {
        let _ = self
            .instance
            .render()
            .inspect_err(|e| warn!("Render pass failed: {:?}", e));
    }

    pub fn destroy(&mut self) {
        self.instance.destroy();
    }
}
