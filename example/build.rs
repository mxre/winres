#[cfg(target_os = "windows")]
extern crate winres;
#[cfg(target_os = "windows")]
use std::io::Write;

#[cfg(target_os = "windows")]
fn main() {
    if std::env::var("PROFILE").unwrap() == "release" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico")
           .set_language(0x0009)
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

#[cfg(not(target_os = "windows"))]
fn main() {
}
