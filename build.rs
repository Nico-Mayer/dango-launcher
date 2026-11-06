fn main() {
    // The launcher draws its own controls; a std-widgets style is still needed
    // for ScrollView and for Palette.color-scheme, which follows the system.
    // The UI tests find elements through debug info, which a release leaves
    // out unless SLINT_EMIT_DEBUG_INFO asks, as for a bundle driven by
    // dango-ui. This setting overrides that variable, so it is read here.
    println!("cargo:rerun-if-env-changed=SLINT_EMIT_DEBUG_INFO");
    let debug_info = std::env::var("PROFILE").as_deref() == Ok("debug")
        || std::env::var_os("SLINT_EMIT_DEBUG_INFO").is_some();
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".into())
        .with_debug_info(debug_info);
    slint_build::compile_with_config("ui/app.slint", config).expect("ui/app.slint compiles");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // The tray links against Common Controls 6, which only a manifest
        // selects. Every artifact gets it, or the test binary fails to load.
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
        println!("cargo:rerun-if-changed=icons/icon.ico");
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("icons/icon.ico");
        resource.set("ProductName", "Dango");
        resource.compile().expect("the Windows resource compiles");
    }
}
