use std::{path::PathBuf, process::Command};

fn main() {
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
    let rcc_status = std::process::Command::new("rcc")
        .arg("-name")
        .arg("resources")
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

    let path = get_qt_include_path();
    cxx_build::bridge("src/main.rs")
        .files(moc_files)
        .flag_if_supported("/Zc:__cplusplus")
        .flag_if_supported("/permissive-") // required for Qt 6 on MSVC
        .include("src")
        .include(&path)
        .include(path.join("QtWidgets"))
        .include(path.join("QtGui"))
        .include(path.join("QtCore"))
        .include(path.join("QtSvg"))
        .std("c++17")
        .compile("window_ffi");
    println!("cargo:rustc-link-search=native={}", path.parent().unwrap().join("lib").display());
    println!("cargo:rustc-link-lib=dylib=Qt6Widgets");
    println!("cargo:rustc-link-lib=dylib=Qt6Gui");
    println!("cargo:rustc-link-lib=dylib=Qt6Core");
    println!("cargo:rustc-link-lib=dylib=Qt6Svg");
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/**/*.h");
    println!("cargo:rerun-if-changed=src/**/*.cpp");
    if cfg!(target_os = "windows") {
        generate_windows_resources();
    }
}

fn get_qt_include_path() -> PathBuf {
    let output = Command::new("qmake") 
        .arg("-query")
        .arg("QT_INSTALL_HEADERS")
        .output()
        .expect("Failed to execute qmake");
    if !output.status.success() {
        panic!("qmake failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let path = String::from_utf8(output.stdout).expect("Failed to parse qmake output");
    let path = PathBuf::from(path.trim());
    if !path.exists() {
        panic!("Qt include path does not exist: {}", path.display());
    }
    path
}

fn generate_windows_resources() {
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
