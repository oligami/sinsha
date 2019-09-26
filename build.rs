use std::process::Command;
use std::path::Path;

fn main() {
    let shaders = [
        ["src/vulkan/render/dim3/gui.vert", "src/vulkan/render/dim3/vert.spv"],
        ["src/vulkan/render/dim3/gui.frag", "src/vulkan/render/dim3/frag.spv"],
        ["src/vulkan/render/lighting/vert.vert", "src/vulkan/render/lighting/vert.spv"],
        ["src/vulkan/render/lighting/frag.frag", "src/vulkan/render/lighting/frag.spv"],
    ];

    shaders.iter()
        .for_each(|[source, spv]| {
            let output = Command::new("glslangValidator")
                .args(&["-V", source, "-o", spv])
                .output()
                .unwrap();

            println!(
                "\u{001b}[33;1mSPIR-V Compile:\u{001b}[m {}\n{}",
                output.status,
                String::from_utf8(output.stdout).unwrap()
            );
            assert!(output.status.success());
        });
}