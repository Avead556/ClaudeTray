fn main() {
    slint_build::compile("ui/app.slint").expect("Slint UI compiles");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        resource.set("ProductName", "ClaudeTray");
        resource.set("FileDescription", "ClaudeTray");
        resource.compile().expect("Windows resources compile");
    }
}
