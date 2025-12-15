use std::{env, path::PathBuf, process::Command};

fn main() {
    if let Some(mut vulkan_sdk) = env::var_os("VULKAN_SDK") {
        vulkan_sdk.push("/bin");
        let path: PathBuf = vulkan_sdk.into();
        if path.exists() {
            println!("Compiling shaders in {path:?}");
            Command::new("glslc")
                .current_dir(path)
                .status()
                .unwrap();
        } else {
            println!("VULKAN_SDK env variable found, but specified folder does not exists.");
        }
    } else {
        println!("VULKAN_SDK env variable not found, skipping shader compilation step.");
    }
}
