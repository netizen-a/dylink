use std::ffi::{CStr, c_void};
use std::marker::PhantomData;
use std::sync::Mutex;
use crate::loader::{LibHandle, Loader};
use once_cell::sync::OnceCell;
use crate::FnAddr;
use crate::vk;

#[derive(Debug)]
pub struct LazyLib<L: Loader, const N: usize>{
    once: OnceCell<()>,
    libs: [&'static CStr; N],
    hlib: Mutex<Vec<(&'static CStr, LibHandle<'static, c_void>)>>,
    phtm: PhantomData<L>,
}

impl <L: Loader, const N: usize> LazyLib<L, N> {
    pub const fn new(libs: [&'static CStr; N]) -> Self {
        Self{
            once: OnceCell::new(),
            libs,
            hlib: Mutex::new(Vec::new()),
            phtm: PhantomData
        }
    }
    pub fn find_sym(&self, sym: &'static CStr) -> crate::FnAddr
	where
		L::Data: 'static + Send,
	{		
		for lib in self.libs.iter() {
			let sym = self.load_and_bind(lib, sym);
			if !sym.is_null() {
				return sym;
			}
		}
		std::ptr::null()
	}
    
    pub fn vk_find_sym(&self, sym: &'static CStr) -> crate::FnAddr
	where
		L::Data: 'static + Send,
	{
		vk::vulkan_loader(sym)
	}

    /// loads function from library synchronously and binds library handle internally to dylink.
    /// 
    /// If the library is already bound, the bound handle will be used for loading the function.
    fn load_and_bind(&self, lib_name: &'static CStr, fn_name: &'static CStr) -> FnAddr
    where
    	L::Data: 'static + Send,
    {
    	let fn_addr: FnAddr;
    	let lib_handle: LibHandle::<L::Data>;
    	let mut lock = self.hlib.lock().unwrap();
    	match lock.binary_search_by_key(&lib_name, |(k, _)| k) {
    		Ok(index) => {
    			lib_handle = LibHandle::from_opaque(&lock[index].1);
    			fn_addr = L::load_sym(&lib_handle, fn_name)
    		}
    		Err(index) => {
    			lib_handle = L::load_lib(lib_name);
    			if lib_handle.is_invalid() {
    				return std::ptr::null();
    			} else {
    				fn_addr = L::load_sym(&lib_handle, fn_name);
    				lock.insert(index, (lib_name, lib_handle.to_opaque()));
    			}
    		}
    	}
    	fn_addr
    }
}