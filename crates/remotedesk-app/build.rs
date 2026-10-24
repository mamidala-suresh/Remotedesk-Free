fn main() {
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=app.manifest");

    #[cfg(windows)]
    {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let res_path = std::path::Path::new(&out_dir).join("resource.o");
        let status = std::process::Command::new("windres")
            .args(&["app.rc", "-O", "coff", "-o"])
            .arg(&res_path)
            .status();

        if let Ok(s) = status {
            if s.success() {
                println!("cargo:rustc-link-arg={}", res_path.display());
            }
        }
    }
}
