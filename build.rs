use std::process::Command;
use std::path::{ PathBuf };

fn main() {
    let shader_root = PathBuf::from("src/vulkan/render/");

    let shaders = [
        "dim3",
        "lighting",
        "gui_rect_2d",
        "gui_3d",
    ];

    shaders
        .iter()
        .map(|shader| {
            let mut glsl_vert = shader_root.clone();
            glsl_vert.push(shader);
            glsl_vert.push("glsl.vert");

            let mut glsl_frag = shader_root.clone();
            glsl_frag.push(shader);
            glsl_frag.push("glsl.vert");

            let mut vert_spv = shader_root.clone();
            vert_spv.push(shader);
            vert_spv.push("vert.spv");

            let mut frag_spv = shader_root.clone();
            frag_spv.push(shader);
            frag_spv.push("frag.spv");

            [glsl_vert, glsl_frag, vert_spv, frag_spv]
        })
        .for_each(|[glsl_vert, glsl_frag, vert_spv, frag_spv]| {
            [[glsl_vert, vert_spv], [glsl_frag, frag_spv]].iter()
                .for_each(|[glsl, spv]| {
                    let output = Command::new("glslangValidator")
                        .args(&["-V", glsl.to_str().unwrap(), "-o", spv.to_str().unwrap()])
                        .output()
                        .unwrap();

                    println!(
                        "\u{001b}[33;1mSPIR-V Compile:\u{001b}[m {}\n{}",
                        output.status,
                        String::from_utf8(output.stdout).unwrap()
                    );
                    assert!(output.status.success());
                })
        });
}