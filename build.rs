use anyhow::{bail, Context};
use glob::glob;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let out_dir = Path::new(&std::env::var("OUT_DIR").unwrap()).to_path_buf();

    // Compile shaders in src/shaders/* to OUT_DIR/shaders/*.spv

    let shader_src_dir = Path::new("src/shaders/");
    let shader_out_dir = out_dir.join("shaders");
    let mut compiler = shaderc::Compiler::new().context("cannot instantiate compiler")?;

    println!(
        "cargo:rerun-if-changed={}",
        shader_src_dir.to_str().context("path is not valid UTF-8")?
    );

    std::fs::create_dir_all(&shader_out_dir).context("cannot create shader output directory")?;

    for src_path_result in glob("src/shaders/**/*")? {
        let src_path = match src_path_result {
            Ok(path) => path,
            Err(err) => {
                eprintln!("Cannot access path: {:?}; skipping", err);
                continue;
            }
        };
        if src_path.is_dir() {
            continue;
        }
        println!(
            "cargo:rerun-if-changed={}",
            src_path.to_str().context("path is not valid UTF-8")?
        );

        let extension = src_path
            .extension()
            .and_then(|s| s.to_str())
            .with_context(|| {
                format!(
                    "Source file {:?} has no extension: expected .vert, .frag or .comp",
                    src_path
                )
            })?;

        let kind = match extension {
            "vert" => shaderc::ShaderKind::Vertex,
            "frag" => shaderc::ShaderKind::Fragment,
            "comp" => shaderc::ShaderKind::Compute,
            _ => bail!(
                "unsupported file extension {:?} (expected .vert, .frag, or .comp)",
                extension
            ),
        };

        let relative_path = src_path.strip_prefix(shader_src_dir).with_context(|| {
            format!(
                "bad prefix of path {:?} (expected {:?})",
                src_path, shader_src_dir,
            )
        })?;
        let out_path = shader_out_dir
            .join(relative_path)
            .with_extension(format!("{}.spv", extension));

        process_shader(&mut compiler, &src_path, &out_path, kind)
            .with_context(|| format!("{:?}: unable to process shader", src_path))?;
    }

    Ok(())
}

fn process_shader(
    compiler: &mut shaderc::Compiler,
    src_path: &Path,
    out_path: &Path,
    shader_kind: shaderc::ShaderKind,
) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(&src_path).context("cannot read shader source")?;

    let artifact = compiler
        .compile_into_spirv(
            &source,
            shader_kind,
            &src_path.to_string_lossy(),
            "main",
            None,
        )
        .context("failed to parse shader source")?;
    std::fs::write(out_path, artifact.as_binary_u8()).context("failed to write shader binary")?;

    Ok(())
}
