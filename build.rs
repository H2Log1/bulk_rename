fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon/icon.ico");
        res.compile().unwrap();
    }
    // 触发更新机制
    println!("cargo:rerun-if-changed=icon/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
}