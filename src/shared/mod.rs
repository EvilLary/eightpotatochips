pub mod shm;
pub mod ticker;

#[inline]
pub fn syscall(res: i32) -> std::io::Result<i32> {
    if res == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(res)
    }
}

#[inline]
pub fn syscall_ptr(ptr: *mut libc::c_void) -> std::io::Result<*mut libc::c_void> {
    if ptr.is_null() {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(ptr)
    }
}
