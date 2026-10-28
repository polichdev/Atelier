use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=native/xaml_bridge.cpp");
    println!("cargo:rerun-if-changed=native/xaml_probe.cpp");
    println!("cargo:rerun-if-changed=native/xaml_probe.manifest");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let revision = std::fs::read("native/xaml_bridge.cpp")
        .unwrap()
        .iter()
        .fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ *b as u64).wrapping_mul(0x100000001b3)
        });
    let compiler = cc::Build::new().cpp(true).get_compiler();
    let status = compiler
        .to_command()
        .args([
            "/nologo",
            "/LD",
            "/MT",
            "/O2",
            "/EHsc",
            "/std:c++17",
            "/DUNICODE",
            "/D_UNICODE",
        ])
        .arg("native/xaml_bridge.cpp")
        .arg(format!("/DATELIER_BRIDGE_REVISION=0x{revision:016x}ULL"))
        .arg(format!("/Fo{}", out.join("xaml_bridge.obj").display()))
        .arg(format!("/Fd{}", out.join("xaml_bridge.pdb").display()))
        .arg("/link")
        .arg(format!("/OUT:{}", out.join("atelier_xaml.dll").display()))
        .arg(format!(
            "/IMPLIB:{}",
            out.join("atelier_xaml.lib").display()
        ))
        .args(["windowsapp.lib", "user32.lib", "ole32.lib", "oleaut32.lib"])
        .status()
        .expect("Could not launch the Visual C++ compiler for the Atelier XAML bridge");
    assert!(status.success(), "Could not build the Atelier XAML bridge");
    if env::var_os("CARGO_FEATURE_XAML_PROBE").is_some() {
        let status = compiler
            .to_command()
            .args([
                "/nologo",
                "/MT",
                "/O2",
                "/EHsc",
                "/std:c++17",
                "/DUNICODE",
                "/D_UNICODE",
            ])
            .arg("native/xaml_probe.cpp")
            .arg(format!("/Fo{}", out.join("xaml_probe.obj").display()))
            .arg(format!("/Fd{}", out.join("xaml_probe.pdb").display()))
            .arg(format!("/Fe{}", out.join("xaml_probe.exe").display()))
            .arg("/link")
            .args([
                "/MANIFEST:EMBED",
                "/MANIFESTINPUT:native/xaml_probe.manifest",
            ])
            .args(["windowsapp.lib", "user32.lib", "ole32.lib", "oleaut32.lib"])
            .status()
            .expect("Could not build the XAML probe");
        assert!(status.success(), "Could not build the XAML probe");
    }
}
