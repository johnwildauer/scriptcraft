use std::{env, process};

fn main() {
    let target_os = match env::var("CARGO_CFG_TARGET_OS") {
        Ok(target_os) => target_os,
        Err(error) => {
            eprintln!("could not determine target OS for Windows icon embedding: {error}");
            process::exit(1);
        }
    };

    if target_os != "windows" {
        return;
    }

    println!("cargo:rerun-if-changed=ScriptCraft.rc");
    println!("cargo:rerun-if-changed=ScriptCraft.manifest");
    println!("cargo:rerun-if-changed=../../packaging/resources/AppIcon.ico");

    if let Err(error) =
        embed_resource::compile("ScriptCraft.rc", embed_resource::NONE).manifest_optional()
    {
        eprintln!("failed to embed the ScriptCraft Windows resources: {error}");
        process::exit(1);
    }
}
