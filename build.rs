fn main() {
    if cfg!(target_os = "windows") {    
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/app.ico");
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductName", "Vibrance");
        res.set(
            "FileDescription",
            &format!("Vibrance"),
        );
        res.compile().expect("Failed to compile Windows resources");
    }
}