use super::*;


impl<L: Loader> LibraryGuard<'_, L> {
    /// Attempts to resolve lazily evaluated library handle, which if successful will also
    /// attempt to resolve symbol. If a symbol is resolved successfully, `psym` will swap with
    /// ordering [`SeqCst`](Ordering::SeqCst) and resolved symbol.
    /// If successful the return value is Some with last address in `psym`, otherwise returns None.
	pub fn find_and_swap(&mut self, psym: &AtomicPtr<()>, symbol: &str) -> Option<SymAddr> {
        if let None = *self.guard {
			*self.guard = unsafe {self.libs.iter().find_map(|name| L::open(name))};
		}

		if let Some(ref lib_handle) = *self.guard {
			let sym = unsafe { L::find_symbol(lib_handle, symbol) };
			if sym.is_null() {
				None
			} else {
				Some(psym.swap(sym.cast_mut(), Ordering::SeqCst))
			}
		} else {
			None
		}
	}
}

impl<L: Close> CloseableLibraryGuard<'_, L> {
    /// Attempts to resolve lazily evaluated library handle, which if successful will also
    /// attempt to resolve symbol. If a symbol is resolved successfully, `psym` will swap with
    /// ordering [`SeqCst`](Ordering::SeqCst) and resolved symbol.
    /// If successful the return value is Some with last address in `psym`, otherwise returns None.
    ///
	/// The last symbol and the atomic variable will be stored internally to be reset to initial state
	/// when [`close`](CloseableLibraryGuard::close) is called
	pub fn find_and_swap(&mut self, psym: &'static AtomicPtr<()>, symbol: &str) -> Option<SymAddr> {
		if let None = self.guard.0 {
			self.guard.0 = unsafe {self.libs.iter().find_map(|name| L::open(name))};
		}

		if let Some(ref lib_handle) = self.guard.0 {
			let sym = unsafe { L::find_symbol(lib_handle, symbol) };
			if sym.is_null() {
				None
			} else {
				let last_symbol = psym.swap(sym.cast_mut(), Ordering::SeqCst);
				self.guard.1.push((psym, AtomicSymAddr::new(last_symbol)));
				Some(last_symbol)
			}
		} else {
			None
		}
	}
    /// Closes, but does not `drop` the library.
    ///
    /// All associated function pointers are reset to initial state.
	///
	/// # Errors
	/// This may error if library is uninitialized.
	pub unsafe fn close(&mut self) -> io::Result<()> {
		let (hlib, rstv) = &mut *self.guard;
		if let Some(handle) = hlib.take() {
			for (pfn, init_addr) in rstv.drain(..) {
				pfn.store(init_addr.into_inner(), Ordering::Release);
			}
			match handle.close() {
				Ok(()) => Ok(()),
				Err(e) => Err(e),
			}
		} else {
			Err(io::Error::new(
				io::ErrorKind::InvalidInput,
				"`CloseableLibrary` is uninitialized.",
			))
		}
	}
}