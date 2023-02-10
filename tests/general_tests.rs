use dylink::dylink;

#[cfg(windows)]
#[test]
fn test_win32_kernel32() {
	use windows_sys::Win32::Foundation::{ERROR_INVALID_PRINTER_COMMAND, WIN32_ERROR};
	#[dylink(name = "Kernel32.dll")]
	extern "stdcall" {
		fn SetLastError(_: WIN32_ERROR);
		fn GetLastError() -> WIN32_ERROR;
	}
	unsafe {
		SetLastError(ERROR_INVALID_PRINTER_COMMAND);
		assert_eq!(GetLastError(), ERROR_INVALID_PRINTER_COMMAND);
	}
}
