use std::{cell::LazyCell, fs, path::Path, time::Instant};

use shaderc::Compiler;

fn main() {
    let t0 = Instant::now();
    let resources = "../base/resources";
    let shaders = format!("{}/shaders", resources);
    //println!("cargo::rerun-if-changed={}", resources);
    println!("cargo::rerun-if-changed={}", shaders);

    let compiler: LazyCell<Compiler> = LazyCell::new(|| shaderc::Compiler::new().unwrap());
    let path = Path::new(&shaders);
    if path.exists() {
        compile_folder(path, &compiler);
    }
    let time = Instant::now() - t0;
    println!("Shaders processed in {:?}", time);
}

fn compile_folder(path: &Path, compiler: &LazyCell<Compiler>) {
    match fs::read_dir(path) {
        Ok(read) => {
            for entry in read {
                let file = entry.unwrap();
                let path = file.path();

                if path.is_dir() {
                    compile_folder(&path, compiler);
                } else if path
                    .extension()
                    .map_or(false, |ext| ext == "vert" || ext == "frag")
                {
                    compile_file(&path, compiler);
                }
            }
        }
        Err(e) => println!("cargo::error={}", e),
    }
}

fn compile_file(path: &Path, compiler: &LazyCell<Compiler>) {
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
    let artifact = compiler.compile_into_spirv(&shader_source, kind, file_name, "main", None);

    match artifact {
        Ok(binary) => {
            let dir = path.parent().unwrap();
            //let out_dir = std::env::var("OUT_DIR").unwrap();
            let dest_path = Path::new(&dir).join(format!("{}.spv", file_name));

            fs::write(&dest_path, binary.as_binary_u8()).unwrap();
        }
        Err(err) => {
            panic!("Shader compilation failed: {}", err);
        }
    }
}
