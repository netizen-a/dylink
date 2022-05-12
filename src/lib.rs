// Copyright (c) 2020 Jonathan "Razordor" Alan Thomason

//re-export the dylink macro
pub extern crate dylink_macro;
pub use dylink_macro::dylink;

//perpetuate dependency for macro
#[doc(hidden)]
pub use once_cell::sync::Lazy;

use std::cell::UnsafeCell;
use std::ffi;
use std::os::{
    raw::c_char,
    // FIXME: this type violates strict-provenance
    //windows::raw::HANDLE,
};
use std::ptr;
use std::mem;
use std::sync::{
    atomic::{AtomicPtr, Ordering},
    Mutex,
};
use windows::Win32::Graphics::OpenGL as gl;
use windows::core;
use windows::Win32::System::LibraryLoader::{LoadLibraryA, GetProcAddress};
use windows::Win32::Foundation::HINSTANCE;

// The loader functions can be called on different threads by the user,
// therefore the following precautions, namely Mutex for thread safety are necessary.
type DllHandle = Mutex<UnsafeCell<Vec<(String, HINSTANCE)>>>;
static DLL_DATA: Lazy<DllHandle> = Lazy::new(|| Mutex::new(UnsafeCell::new(Vec::new())));

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct DispatchableHandle(*mut ffi::c_void);
impl DispatchableHandle {
    #[inline]
    pub const fn null() -> Self {
        Self(ptr::null_mut())
    }
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

/// Context is used in place of `VkInstance` to invoke the vulkan specialization.
// VkInstance and VkDevice are both dispatchable and therefore cannonically 64-bit integers
pub struct Context {
    instance: AtomicPtr<ffi::c_void>,
    device: AtomicPtr<ffi::c_void>,
}

impl Context {
    // 'new' is used to initialize the static variable
    pub const fn new() -> Self {
        Self {
            instance: AtomicPtr::new(ptr::null_mut()),
            device: AtomicPtr::new(ptr::null_mut()),
        }
    }
    /// Setting instance allows dylink to load Vulkan functions.
    #[inline]
    pub fn set_instance<T: Into<DispatchableHandle>>(&self, inst: T) {
        self.instance.store(inst.into().0, Ordering::Relaxed);
    }
    /// Setting device to a non-null value lets Dylink call `vkGetDeviceProcAddr`.    
    #[inline]
    pub fn set_device<T: Into<DispatchableHandle>>(&self, dev: T) {
        self.device.store(dev.into().0, Ordering::Relaxed);
    }
    #[inline]
    pub fn get_instance(&self) -> DispatchableHandle {
        DispatchableHandle(self.instance.load(Ordering::Relaxed))
    }
    #[inline]
    pub fn get_device(&self) -> DispatchableHandle {
        DispatchableHandle(self.device.load(Ordering::Relaxed))
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Self {
            instance: AtomicPtr::new(self.instance.load(Ordering::Relaxed)),
            device: AtomicPtr::new(self.device.load(Ordering::Relaxed)),
        }
    }
}

unsafe impl Sync for Context {}

pub static CONTEXT: Context = Context::new();

/// `vkloader` is a vulkan loader specialization.
/// vulkan 1.2 or above is recommended.
pub fn vkloader(fn_name: &str, context: Context) -> ptr::NonNull<ffi::c_void> {
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    static vkGetInstanceProcAddr: Lazy<extern "stdcall" fn(DispatchableHandle, *const c_char) -> *const ffi::c_void> =
        Lazy::new(|| unsafe {
            std::mem::transmute(loader("vulkan-1.dll", "vkGetInstanceProcAddr"))
        });
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    static vkGetDeviceProcAddr: Lazy<extern "stdcall" fn(DispatchableHandle, *const c_char) -> *const ffi::c_void> =
        Lazy::new(|| unsafe {
            std::mem::transmute(vkGetInstanceProcAddr(
                DispatchableHandle(CONTEXT.instance.load(Ordering::Relaxed)),
                "vkGetDeviceProcAddr\0".as_ptr() as *const c_char,
            ))
        });

    let addr = {
        let c_fn_name = ffi::CString::new(fn_name).unwrap();
        let device = context.get_device();
        if device.is_null() {
            vkGetInstanceProcAddr(context.get_instance(), c_fn_name.as_ptr())
        } else {
            let addr = vkGetDeviceProcAddr(device, c_fn_name.as_ptr());
            if addr == std::ptr::null() {
                #[cfg(debug_assertions)]
                println!("Dylink Warning: `{}` not found using `vkGetDeviceProcAddr`. Deferring call to `vkGetInstanceProcAddr`.", fn_name);
                vkGetInstanceProcAddr(context.get_instance(), c_fn_name.as_ptr())
            } else {
                addr
            }
        }
    };
    ptr::NonNull::new(addr as *mut _).expect(&format!("Dylink Error: `{fn_name}` not found!"))
}

/// `glloader` is an opengl loader specialization.
pub fn glloader(fn_name: &str) -> *const ffi::c_void {
    let addr = unsafe {
        let c_fn_name = ffi::CString::new(fn_name).unwrap();
        gl::wglGetProcAddress(core::PCSTR(c_fn_name.as_ptr() as *const _)).expect(&format!("Dylink Error: `{fn_name}` not found!"))
    };    
    unsafe {mem::transmute(addr)}
}

/// `loader` is a generalization for all other dlls.
pub fn loader(lib_name: &str, fn_name: &str) -> ptr::NonNull<ffi::c_void> {
    let mut lib_handle = HINSTANCE::default();
    let mut lib_found = false;
    unsafe {
        let dll_data = DLL_DATA.lock().unwrap().get();
        for lib_set in (*dll_data).iter_mut() {
            if lib_set.0 == lib_name {
                lib_found = true;
                lib_handle = lib_set.1;
            }
        }

        if !lib_found {
            let lib_cstr = ffi::CString::new(lib_name).unwrap();
            lib_handle = LoadLibraryA(core::PCSTR(lib_cstr.as_ptr() as *const _)).expect("Dylink Error: `{}` not found!");
            (*dll_data).push((lib_name.to_string(), lib_handle));
        }
    }

    let fn_cstr = ffi::CString::new(fn_name).unwrap();

    let addr: *const ffi::c_void =
        unsafe { std::mem::transmute(GetProcAddress(lib_handle, core::PCSTR(fn_cstr.as_ptr() as *const _))) };
    ptr::NonNull::new(addr as *mut _).expect(&format!("Dylink Error: `{fn_name}` not found!"))
}
