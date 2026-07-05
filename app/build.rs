use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use pathdiff::diff_paths;

struct CompilerContext {
    sdk_bin_dir: PathBuf,
    base_dir: PathBuf,
    dst_dir: PathBuf,
}

impl CompilerContext {
    fn new(base_dir: &Path, dst_dir: &Path, sdk_bin_dir: &Path) -> Self {
        Self {
            sdk_bin_dir: sdk_bin_dir.to_owned(),
            base_dir: base_dir.to_owned(),
            dst_dir: dst_dir.to_owned(),
        }
    }

    fn compile_folder(&self, path: &Path) {
        match fs::read_dir(path) {
            Ok(read) => {
                for entry in read {
                    let file = entry.unwrap();
                    let path = file.path();

                    if path.is_dir() {
                        self.compile_folder(&path);
                    } else if path
                        .extension()
                        .map_or(false, |ext| ext == "vert" || ext == "frag")
                    {
                        self.compile_file(&path);
                    }
                }
            }
            Err(e) => println!("cargo::error={}", e),
        }
    }

    fn compile_file(&self, path: &Path) {
        println!(
            "cargo::warning=Compiling shader: {}",
            path.to_string_lossy()
        );

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap();
        let file_dir = path.parent().unwrap();
        let relative = diff_paths(file_dir, &self.base_dir).unwrap();
        let dest_path = Path::new(&self.dst_dir)
            .join(relative)
            .join(format!("{}.spv", file_name));

        println!(
            "cargo::warning=Saving compiled shader to {}",
            dest_path.to_string_lossy()
        );
        let status = Command::new("glslc")
            .current_dir(&self.sdk_bin_dir)
            .arg(path)
            .arg("-o")
            .arg(&dest_path)
            .status()
            .expect("Unable to run glslc. Ensure VULKAN_SDK environment variable is set!");

        if !status.success() {
            panic!("Compilation failed: {}", path.to_string_lossy());
        }
    }
}

fn main() {
    let t0 = Instant::now();
    let resources = "../base/resources";
    let res_dir = to_absolute_path(resources);
    let shaders = format!("{}/shaders", &resources);
    let shaders = to_absolute_path(&shaders);
    //println!("cargo::rerun-if-changed={}", resources);
    println!("cargo::rerun-if-changed={}", shaders.to_str().unwrap());

    let out_dir = env::var("OUT_DIR").unwrap();
    let mut dst_dir = to_absolute_path(&out_dir);
    while dst_dir.file_name().unwrap() != "target" {
        let _ = dst_dir.pop();
    }

    let mut vulkan_sdk = env::var_os("VULKAN_SDK").expect("VULKAN_SDK is not set!");
    vulkan_sdk.push("/bin");
    let sdk_bin_dir = Path::new(&vulkan_sdk);

    println!(
        "Using res_dir={}, dst_dir={}, sdk_bin_dir={}",
        res_dir.to_string_lossy(),
        dst_dir.to_string_lossy(),
        sdk_bin_dir.to_string_lossy()
    );

    let ctx = CompilerContext::new(&res_dir, &dst_dir, &sdk_bin_dir);
    ctx.compile_folder(&shaders);

    let time = Instant::now() - t0;
    println!("Shaders processed in {:?}", time);
}

const WIN_PREFIX: &str = r"\\?\";

fn to_absolute_path(value: &str) -> PathBuf {
    let mut canonic = Path::new(value)
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    if canonic.starts_with(WIN_PREFIX) {
        canonic = canonic.strip_prefix(WIN_PREFIX).unwrap().into();
    }
    return PathBuf::from(canonic);
}
