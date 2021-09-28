extern crate winres;

fn main() {
    // only run if target os is windows
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() != "windows" {
        return;
    }

    // only build the resource for release builds
    // as calling rc.exe might be slow
    if std::env::var("PROFILE").unwrap() == "release" {
        let mut res = winres::WindowsResource::new();
        if cfg!(unix) {
            // paths for X64 on archlinux
            res.set_toolkit_path("/usr/x86_64-w64-mingw32/bin");
            // ar tool for mingw in toolkit path
            res.set_ar_path("ar");
            // windres tool
            res.set_windres_path("/usr/bin/x86_64-w64-mingw32-windres");
        }

        res.set_icon("icon.ico")
            // can't use winapi crate constants for cross compiling
            // MAKELANGID(LANG_ENGLISH, SUBLANG_ENGLISH_US )
            .set_language(0x0409)
            .set_manifest_file("manifest.xml");
        if let Err(e) = res.compile() {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
