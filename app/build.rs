use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use pathdiff::diff_paths;
use shaderc::Compiler;

struct CompilerContext {
    base_dir: PathBuf,
    dst_dir: PathBuf,
    compiler: Compiler,
}

impl CompilerContext {
    fn new(base_dir: &Path, dst_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_owned(),
            dst_dir: dst_dir.to_owned(),
            compiler: shaderc::Compiler::new().unwrap(),
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
        println!("cargo::warning=Compiling shader: {:?}", path);

        let kind = path
            .extension()
            .map(|ext| {
                if ext == "vert" {
                    shaderc::ShaderKind::Vertex
                } else {
                    shaderc::ShaderKind::Fragment
                }
            })
            .unwrap();

        let shader_source = fs::read_to_string(&path).unwrap();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap();
        let artifact =
            self.compiler
                .compile_into_spirv(&shader_source, kind, file_name, "main", None);

        match artifact {
            Ok(binary) => {
                let dir = path.parent().unwrap();
                let relative = diff_paths(dir, &self.base_dir).unwrap();
                let dest_path = Path::new(&self.dst_dir)
                    .join(relative)
                    .join(format!("{}.spv", file_name));

                println!("cargo::warning=Saving compiled shader to {:?}", &dest_path);
                fs::create_dir_all(dest_path.parent().unwrap()).unwrap();
                fs::write(&dest_path, binary.as_binary_u8()).unwrap();
            }
            Err(err) => {
                panic!("Shader compilation failed: {}", err);
            }
        }
    }
}

fn main() {
    let t0 = Instant::now();
    let resources = "../base/resources";
    let shaders = format!("{}/shaders", resources);
    //println!("cargo::rerun-if-changed={}", resources);
    println!("cargo::rerun-if-changed={}", shaders);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let mut dst_dir = PathBuf::from(&out_dir);
    while dst_dir.file_name().unwrap() != "target" {
        let _ = dst_dir.pop();
    }
    let ctx = CompilerContext::new(Path::new(&resources), &dst_dir);
    ctx.compile_folder(Path::new(&shaders));

    let time = Instant::now() - t0;
    println!("Shaders processed in {:?}", time);
}