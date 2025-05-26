use std::env;

fn main() {
    let mut config = cpp_build::Config::new();
    let mut res = winres::WindowsResource::new();
    // res.set_icon("resources/app_icon.ico");
    res.set("FileVersion", env!("CARGO_PKG_VERSION"));
    res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
    res.set("ProductName", "Vibrance");
    res.set("FileDescription", &format!("Vibrance v{}", env!("CARGO_PKG_VERSION")));
    res.compile().unwrap();
    config.build("src/main.rs");
}