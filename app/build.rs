use std::{env, process::Command};

fn main() {
    if let Some(mut vulkan_sdk) = env::var_os("VULKAN_SDK") {
        vulkan_sdk.push("/bin");
        Command::new("glsc").current_dir(vulkan_sdk).status().unwrap();
    } else {
        println!("VULKAN_SDK env variable not found, skipping shader compilation step.");
    }
}
