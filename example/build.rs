#[cfg(target_os = "windows")]
extern crate winres;
#[cfg(target_os = "windows")]
extern crate winapi;

// only build for windows
#[cfg(target_os = "windows")]
fn main() {
    use std::io::Write;
    // only build the resource for release builds
    // as calling rc.exe might be slow
    if std::env::var("PROFILE").unwrap() == "release" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico")
           .set_language(
               winapi::um::winnt::MAKELANGID(
                   winapi::um::winnt::LANG_ENGLISH,
                   winapi::um::winnt::SUBLANG_ENGLISH_US
                )
            )
           .set_manifest_file("manifest.xml");
        match res.compile() {
            Err(e) => {
                write!(std::io::stderr(), "{}", e).unwrap();
                std::process::exit(1);
            }
            Ok(_) => {}
        }
    }
}

// nothing to do for other operating systems
#[cfg(not(target_os = "windows"))]
fn main() {
}
