fn main() {
    // Fluent on both platforms, so the launcher looks the same everywhere. The
    // UI tests find elements through debug info, which a release leaves out.
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".into())
        .with_debug_info(std::env::var("PROFILE").as_deref() == Ok("debug"));
    slint_build::compile_with_config("ui/app.slint", config).expect("ui/app.slint compiles");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // The tray links against Common Controls 6, which only a manifest
        // selects. Every artifact gets it, or the test binary fails to load.
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("icons/icon.ico");
        resource.set("ProductName", "Dango");
        resource.compile().expect("the Windows resource compiles");
    }
}
