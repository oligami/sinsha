use std::process::Command;

fn main() {
    let shaders = [
        ["shaders/gui/gui.vert", "shaders/gui/vert.spv"],
        ["shaders/gui/gui.frag", "shaders/gui/frag.spv"],
        ["shaders/test/vert.vert", "shaders/test/vert.spv"],
        ["shaders/test/frag.frag", "shaders/test/frag.spv"],
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