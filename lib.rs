//! Rust Windows resource helper
//!
//! This crate implements a simple generator for Windows resource (.rc) files
//! for use with either Microsoft `rc.exe` resource compiler or with GNU `windres.exe`
//!
//! The `WindowsResorce::compile()` method is inteded to be used from a build script and
//! needs environment variables from cargo to be set. It not only compiles the resource
//! but directs cargo to link the resource compilers output.
//!
//! # Example
//!
//! ```rust,no_run
//! # extern crate winres;
//! if cfg!(target_os = "windows") {
//!     let mut res = winres::WindowsResource::new();
//!     res.set_icon("icon.ico")
//!        .set("InternalName", "TEST.EXE")
//!        .set_version_info(winres::VersionInfoProperty::PRODUCTVERSION, 0x0001000000000000);
//!     res.compile().unwrap();
//! }
//! ```
//!
//! # Defaults
//!
//! We try to guess some sensible default values from Cargo's build time environement variables
//! This is described in `WindowsResource::new()`. Further more we have to know there to find the
//! resource compiler for the MSVC Toolkit this can be done by looking up a registry key but
//! for MinGW this has to be done manually.
//!
//! The following paths are the hardcoded defaults:
//! MSVC the last registry key at
//! `HKLM\SOFTWARE\Microsoft\Windows Kits\Installed Roots`, for MinGW we try our luck by simply
//! using the `%PATH%` environment variable.
//!
//! Note that the toolkit bitness as to match the one from the current Rust compiler. If you are
//! using Rust GNU 64-bit you have to use MinGW64. For MSVC this is simpler as (recent) Windows
//! SDK always installs both versions on a 64-bit system.

use std::env;
use std::path::{PathBuf, Path};
use std::process;
use std::collections::HashMap;
use std::io;
use std::io::prelude::*;
use std::fs;
use std::error::Error;

pub enum Toolkit {
    MSVC,
    GNU,
    Unknown,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum VersionInfo {
    /// Fileversion
    ///
    /// The version value consists of four 16 bit words, e.g.,
    /// `MAJOR << 48 | MINOR << 32 | PATCH << 16 | RELEASE`
    FILEVERSION,
    /// Productversion
    ///
    /// The version value consists of four 16 bit words, e.g.,
    /// `MAJOR << 48 | MINOR << 32 | PATCH << 16 | RELEASE`
    PRODUCTVERSION,
    /// Targeted operating system
    ///
    /// Should be Windows NT Win32, with value `0x40004`
    FILEOS,
    /// Type of the resulting binary
    ///
    /// The value (for a rust compiler output) should be
    /// 1 for a EXE and 2 for a DLL
    FILETYPE,
    /// Subtype
    ///
    /// Only for Windows drivers
    FILESUBTYPE,
    /// Bit mask for FILEFLAGS
    FILEFLAGSMASK,
    /// Additional file flags
    ///
    /// Only the bits set in FILEFLAGSMASK are read
    FILEFLAGS,
}

pub struct WindowsResource {
    toolkit_path: String,
    properties: HashMap<String, String>,
    version_info: HashMap<VersionInfo, u64>,
    rc_file: Option<String>,
    icon: Option<String>,
    language: u16
}

impl WindowsResource {
    pub fn toolkit() -> Toolkit {
        if cfg!(target_env = "gnu") {
            Toolkit::GNU
        } else if cfg!(target_env = "msvc") {
            Toolkit::MSVC
        } else {
            Toolkit::Unknown
        }
    }

    /// Create a new resource with version info struct
    ///
    ///
    /// We initialize the resource file with values provided by cargo
    ///
    /// | Field                | Cargo / Values               |
    /// |----------------------|------------------------------|
    /// | `"FileVersion"`      | `package.version`            |
    /// | `"ProductVersion"`   | `package.version`            |
    /// | `"ProductName"`      | `package.name`               |
    /// | `"FileDescription"`  | `package.description`        |
    ///
    /// The version info struct is set to some values
    /// sensible for creating an executable file.
    ///
    /// | Property             | Cargo / Values               |
    /// |----------------------|------------------------------|
    /// | `FILEVERSION`        | `package.version`            |
    /// | `PRODUCTVERSION`     | `package.version`            |
    /// | `FILEOS`             | `VOS_NT_WINDOWS32 (0x40004)` |
    /// | `FILETYPE`           | `VFT_APP (0x1)`              |
    /// | `FILESUBTYPE`        | `VFT2_UNKNOWN (0x0)`         |
    /// | `FILEFLAGSMASK`      | `VS_FFI_FILEFLAGSMASK (0x3F)`|
    /// | `FILEFLAGS`          | `0x0`                        |
    ///
    /// Additionally, the language is set to neutral (0),
    /// and no icon is set.
    pub fn new() -> Self {
        let mut props: HashMap<String, String> = HashMap::new();
        let mut ver: HashMap<VersionInfo, u64> = HashMap::new();

        props.insert("FileVersion".to_string(),
                     env::var("CARGO_PKG_VERSION").unwrap().to_string());
        props.insert("ProductVersion".to_string(),
                     env::var("CARGO_PKG_VERSION").unwrap().to_string());
        props.insert("ProductName".to_string(),
                     env::var("CARGO_PKG_NAME").unwrap().to_string());
        props.insert("FileDescription".to_string(),
                     env::var("CARGO_PKG_DESCRIPTION").unwrap().to_string());

        let mut version = 0 as u64;
        version |= env::var("CARGO_PKG_VERSION_MAJOR").unwrap().parse().unwrap_or(0) << 48;
        version |= env::var("CARGO_PKG_VERSION_MINOR").unwrap().parse().unwrap_or(0) << 32;
        version |= env::var("CARGO_PKG_VERSION_PATCH").unwrap().parse().unwrap_or(0) << 16;
        // version |= env::var("CARGO_PKG_VERSION_PRE").unwrap().parse().unwrap_or(0);
        ver.insert(VersionInfo::FILEVERSION, version);
        ver.insert(VersionInfo::PRODUCTVERSION, version);
        ver.insert(VersionInfo::FILEOS, 0x00040004);
        ver.insert(VersionInfo::FILETYPE, 1);
        ver.insert(VersionInfo::FILESUBTYPE, 0);
        ver.insert(VersionInfo::FILEFLAGSMASK, 0x3F);
        ver.insert(VersionInfo::FILEFLAGS, 0);

        let sdk = match get_sdk() {
            Ok(mut v) => v.pop().unwrap(),
            Err(_) => String::new(),
        };

        WindowsResource {
            toolkit_path: sdk,
            properties: props,
            version_info: ver,
            rc_file: None,
            icon: None,
            language: 0,
        }
    }

    /// Set string properties of the version info struct.
    ///
    /// Possible field names are:
    ///
    ///  - `"FileVersion"`
    ///  - `"FileDescription"`
    ///  - `"ProductVersion"`
    ///  - `"ProductName"`
    ///  - `"OriginalFilename"`
    ///  - `"LegalCopyright"`
    ///  - `"LeagalTrademark"`
    ///  - `"CompanyName"`
    ///  - `"Comments"`
    ///  - `"InternalName"`
    ///
    /// Additionally there exists
    /// `"PrivateBuild"`, `"SpecialBuild"`
    /// which should only be set, when the `FILEFLAGS` property is set to
    /// `VS_FF_PRIVATEBUILD(0x08)` or `VS_FF_SPECIALBUILD(0x20)`
    pub fn set<'a>(&mut self, name: &'a str, value: &'a str) -> &mut Self {
        self.properties.insert(name.to_string(), value.to_string());
        self
    }

    /// Set the correct path for the toolkit.
    ///
    /// For the GNU toolkit this has to be the path where MinGW
    /// put `windres.exe` and `ar.exe`. This could be something like:
    /// `"C:\Program Files\mingw-w64\x86_64-5.3.0-win32-seh-rt_v4-rev0\mingw64\bin"`
    ///
    /// For MSVC the Windows SDK has to be installed. It comes with the resource compiler
    /// `rc.exe`. This should be set to the root directory of the Windows SDK, e.g.,
    /// `"C:\Program Files (x86)\Windows Kits\10`
    pub fn set_toolkit_path<'a>(&mut self, path: &'a str) -> &mut Self {
        self.toolkit_path = path.to_string();
        self
    }
    
    /// Set the user interface language of the file
    ///
    /// For possible values take a look at the MSDN page for resource files,
    /// we only included some values here.
    ///
    /// | Language            | Value    |
    /// |---------------------|----------|
    /// | Neutral             | `0x0000` |
    /// | English             | `0x0009` |
    /// | English (US)        | `0x0409` |
    /// | English (GB)        | `0x0809` |
    /// | German              | `0x0407` |
    /// | German (AT)         | `0x0c07` |
    /// | French              | `0x000c` |
    /// | French (FR)         | `0x040c` |
    /// | Catalan             | `0x0003` |
    /// | Basque              | `0x042d` |
    /// | Breton              | `0x007e` |
    /// | Scottish Gaelic     | `0x0091` |
    /// | Romansch            | `0x0017` |
    ///
    /// I know that some of these languages are used less often, but
    /// they were not included in many other lists.
    pub fn set_language(&mut self, language: u16) -> &mut Self {
        self.language = language;
        self
    }

    /// Set an icon filename
    ///
    /// This icon need to be in `ico` format. The filename can be absolute
    /// or relative to the projects root.
    pub fn set_icon<'a>(&mut self, path: &'a str) -> &mut Self {
        self.icon = Some(path.to_string());
        self
    }

    /// Set a version info struct property
    /// Currently we only support numeric values, you have to look them up.
    pub fn set_version_info(&mut self, field: VersionInfo, value: u64) -> &mut Self {
        self.version_info.insert(field, value);
        self
    }

    /// Write a resource file with the set values
    pub fn write_resource_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut f = try!(fs::File::create(path));
        // try!(write!(f, "#include <winver.h>\n"));
        try!(write!(f, "#pragma code_page(65001)\n"));
        try!(write!(f, "1 VERSIONINFO\n"));
        for (k, v) in self.version_info.iter() {
            match *k {
                VersionInfo::FILEVERSION |
                VersionInfo::PRODUCTVERSION => {
                    try!(write!(f,
                                "{:?} {}, {}, {}, {}\n",
                                k,
                                (*v >> 48) as u16,
                                (*v >> 32) as u16,
                                (*v >> 16) as u16,
                                *v as u16))
                }
                _ => try!(write!(f, "{:?} {:#x}\n", k, v)),
            };
        }
        try!(write!(f, "{{\nBLOCK \"StringFileInfo\"\n"));
        try!(write!(f, "{{\nBLOCK \"{:04x}04b0\"\n{{\n", self.language));
        for (k, v) in self.properties.iter() {
            if !v.is_empty() {
                try!(write!(f, "VALUE \"{}\", \"{}\"\n", k, v));
            }
        }
        try!(write!(f, "}}\n}}\n"));

        try!(write!(f, "BLOCK \"VarFileInfo\" {{\n"));
        try!(write!(f, "VALUE \"Translation\", {:#x}, 0x04b0\n", self.language));
        try!(write!(f, "}}\n}}\n"));
        if self.icon.is_some() {
            try!(write!(f, "1 ICON {}\n", self.icon.clone().unwrap()));
        }
        Ok(())
    }

    /// Set an already existing resource file.
    ///
    /// We will neither modify this file nor parse its contents. This function
    /// simply replaces the internaly generated resource file that is passed to
    /// the compiler. You can use this function to write a resource file yourself.
    pub fn set_resource_file<'a>(&mut self, path: &'a str) -> &mut Self {
        self.rc_file = Some(path.to_string());
        self
    }

    #[cfg(target_env = "gnu")]
    fn compile_with_toolkit<'a>(&self, input: &'a str, output_dir: &'a str) -> io::Result<()> {
        let output = PathBuf::from(output_dir).join("resource.o");
        let input = PathBuf::from(input);
        let status = try!(process::Command::new("windres.exe")
            .current_dir(&self.toolkit_path)
            .arg(format!("-I{}", env::var("CARGO_MANIFEST_DIR").unwrap()))
            .arg(format!("{}", input.display()))
            .arg(format!("{}", output.display()))
            .status());
        if !status.success() {
            return Err(io::Error::new(io::ErrorKind::Other, "Could not compile resource file"));
        }

        let libname = PathBuf::from(output_dir).join("libresource.a");
        let status = try!(process::Command::new("ar.exe")
            .current_dir(&self.toolkit_path)
            .arg("rsc")
            .arg(format!("{}", libname.display()))
            .arg(format!("{}", output.display()))
            .status());
        if !status.success() {
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Could not create static library for resource file"));
        }

        println!("cargo:rustc-link-search=native={}", output_dir);
        println!("cargo:rustc-link-lib=static={}", "resource");

        Ok(())
    }

    #[cfg(target_env = "msvc")]
    fn compile_with_toolkit<'a>(&self, input: &'a str, output_dir: &'a str) -> io::Result<()> {
        let rc_exe = if cfg!(target_arch = "x86_64") {
            PathBuf::from(&self.toolkit_path).join("bin\\x64\\rc.exe")
        } else {
            PathBuf::from(&self.toolkit_path).join("bin\\x86\\rc.exe")
        };
        // let inc_win = PathBuf::from(&self.toolkit_path).join("Include\\10.0.10586.0\\um");
        // let inc_shared = PathBuf::from(&self.toolkit_path).join("Include\\10.0.10586.0\\shared");
        let output = PathBuf::from(output_dir).join("resource.lib");
        let input = PathBuf::from(input);
        let status = try!(process::Command::new(rc_exe)
            .arg(format!("/I{}", env::var("CARGO_MANIFEST_DIR").unwrap()))
            //.arg(format!("/I{}", inc_shared.display()))
            //.arg(format!("/I{}", inc_win.display()))
            .arg("/nologo")
            .arg(format!("/fo{}", output.display()))
            .arg(format!("{}", input.display()))
            .status());
        if !status.success() {
            return Err(io::Error::new(io::ErrorKind::Other, "Could not compile resource file"));
        }

        println!("cargo:rustc-link-search=native={}", output_dir);
        println!("cargo:rustc-link-lib=static={}", "resource");
        Ok(())
    }

    /// Run the resource compiler
    ///
    /// This function generates a resource file from the settings, or
    /// uses an existing resource file and passes it to the resource compiler
    /// of your toolkit.
    ///
    /// Further more we will print the correct statements for
    /// `cargo:rustc-link-lib=` and `cargo:rustc-link-search` on the console,
    /// so that the cargo build script can link the compiled resource file.
    pub fn compile(&self) -> io::Result<()> {
        let output = PathBuf::from(env::var("OUT_DIR").unwrap());
        let rc = output.join("resource.rc");
        if self.rc_file.is_none() {
            try!(self.write_resource_file(&rc));
        }
        let rc = self.rc_file.clone().unwrap_or(rc.to_str().unwrap().to_string());
        try!(self.compile_with_toolkit(rc.as_str(), output.to_str().unwrap()));

        Ok(())
    }
}

/// Find a Windows SDK
fn get_sdk() -> Result<Vec<String>, io::Error> {
    let output = try!(process::Command::new("reg")
        .arg("query")
        .arg("HKLM\\SOFTWARE\\Microsoft\\Windows Kits\\Installed Roots")
        .output());

    let lines = try!(String::from_utf8(output.stdout)
        .or_else(|e| Err(io::Error::new(io::ErrorKind::Other, e.description()))));
    let mut kits: Vec<String> = Vec::new();
    for line in lines.lines() {
        if line.trim().starts_with("KitsRoot") {
            kits.push(line.chars()
                .skip(line.find("REG_SZ").unwrap() + 6)
                .skip_while(|c| c.is_whitespace())
                .collect());
        }
    }
    Ok(kits)
}
