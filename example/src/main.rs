extern crate winapi;
extern crate user32;

use std::ffi::CString;
use user32::MessageBoxA;
use winapi::winuser::{MB_OK, MB_ICONINFORMATION};

fn main() {
	let lp_text = CString::new("Hello, world!").unwrap();
    let lp_caption = CString::new("MessageBox Example").unwrap();
	unsafe {
			MessageBoxA(
				std::ptr::null_mut(),
				lp_text.as_ptr(),
				lp_caption.as_ptr(),
				MB_OK | MB_ICONINFORMATION
			);
		}
}
