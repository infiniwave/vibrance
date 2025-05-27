fn main() {
    // TODO: linux support
    
    // execute moc on all header files
    let moc_files = glob::glob("src/**/*.h")
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    for moc_file in moc_files {
        let file = moc_file.with_extension("cpp");
        let file = file.file_name().unwrap().to_str().unwrap();
        let output = std::process::Command::new("moc")
            .arg("-o")
            .arg(format!("src/cpp/moc_{file}"))
            .arg(moc_file)
            .output()
            .expect("Failed to execute moc");
        if !output.status.success() {
            panic!("moc failed: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    // generate C++ source from Qt resource file using rcc
    let rcc_status = std::process::Command::new("C:/Qt/6.9.0/msvc2022_64/bin/rcc")
        .arg("-name").arg("resources")
        .arg("-o")
        .arg("src/cpp/qrc_resources.cpp")
        .arg("resources/resources.qrc")
        .status()
        .expect("Failed to run rcc (Qt Resource Compiler)");
    if !rcc_status.success() {
        panic!("rcc failed");
    }

    // collect all cpp files
    let moc_files = glob::glob("src/**/*.cpp")
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    cxx_build::bridge("src/main.rs")
        .files(moc_files)
        .flag_if_supported("/Zc:__cplusplus")
        .flag_if_supported("/permissive-") // required for Qt 6 on MSVC
        .include("src")
        // TODO: use environment for Qt path
        .include("C:/Qt/6.9.0/msvc2022_64/include")
        .include("C:/Qt/6.9.0/msvc2022_64/include/QtWidgets")
        .include("C:/Qt/6.9.0/msvc2022_64/include/QtGui")
        .include("C:/Qt/6.9.0/msvc2022_64/include/QtCore")
        .std("c++17")
        .compile("window_ffi");
    println!("cargo:rustc-link-search=native=C:/Qt/6.9.0/msvc2022_64/lib");
    println!("cargo:rustc-link-lib=dylib=Qt6Widgets");
    println!("cargo:rustc-link-lib=dylib=Qt6Gui");
    println!("cargo:rustc-link-lib=dylib=Qt6Core");
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/**/*.h");
    println!("cargo:rerun-if-changed=src/**/*.cpp");
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/app.ico");
    res.set("FileVersion", env!("CARGO_PKG_VERSION"));
    res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
    res.set("ProductName", "Vibrance");
    res.set("FileDescription", &format!("Vibrance v{}", env!("CARGO_PKG_VERSION")));
    res.compile().unwrap();
}
