#![allow(unused)]
#[derive(Debug)]
pub(crate) struct ShmData {
    ptr: NonNull<u8>,
    size: usize,
}

impl ShmData {
    pub fn new(size: usize) -> std::io::Result<(Self, RawFd)> {
        let name = unsafe {
            let time = libc::time(core::ptr::null_mut());
            format!("{}\0", time) // replace this ass solution
        };
        let name = name.as_ptr().cast();

        let fd = unsafe {
            let flags = libc::O_RDWR | libc::O_EXCL | libc::O_CREAT;
            syscall(libc::shm_open(name, flags, 0o600))
        }?;

        unsafe {
            syscall(libc::shm_unlink(name))?;
            syscall(libc::ftruncate(fd, size as i64))?;
        }

        let data = unsafe {
            let prot = libc::PROT_READ | libc::PROT_WRITE;
            syscall_ptr(libc::mmap(
                core::ptr::null_mut(),
                size,
                prot,
                libc::MAP_SHARED,
                fd,
                0,
            ))
        }?;

        let ptr = unsafe { NonNull::new_unchecked(data as *mut u8) };

        Ok((Self { ptr, size }, fd))
    }

    pub fn as_slice<T>(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr().cast(), self.size / core::mem::size_of::<T>()) }
    }

    pub fn as_slice_mut<T>(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr().cast(), self.size / core::mem::size_of::<T>()) }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }
}

impl Drop for ShmData {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(
                self.ptr.as_ptr().cast(),
                core::mem::size_of_val(self.ptr.as_mut()),
            );
        }
    }
}

use std::{
    os::fd::{OwnedFd, RawFd},
    ptr::NonNull,
};
use super::{syscall, syscall_ptr};

