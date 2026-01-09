#![allow(unused)]
use super::{syscall, syscall_ptr};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

#[repr(transparent)]
#[derive(Debug)]
pub struct Ticker {
    fd: OwnedFd,
}

impl Ticker {
    /// interval in milis
    pub fn new(interval: u64) -> std::io::Result<Self> {
        let fd = unsafe { syscall(libc::timerfd_create(libc::CLOCK_MONOTONIC, 0)) }?;
        let mut timer_spec = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: 0,
                // why cant i just use 1e9 ???
                tv_nsec: interval.cast_signed() * 1_000_000,
            },
            it_value: libc::timespec {
                tv_sec: 0,
                tv_nsec: 1,
            },
        };
        unsafe {
            syscall(libc::timerfd_settime(
                fd.as_raw_fd(),
                0,
                &raw mut timer_spec,
                std::ptr::null_mut(),
            ))?;

            Ok(Self {
                fd: OwnedFd::from_raw_fd(fd),
            })
        }
    }

    pub fn read_timer(&self) -> std::io::Result<i64> {
        const SIZE: usize = size_of::<i64>();
        let mut buf = [0u8; SIZE];
        unsafe {
            let len = libc::read(
                self.as_raw_fd(),
                buf.as_mut_ptr().cast(),
                SIZE,
            );
            if len != SIZE as isize {
                return Err(std::io::Error::last_os_error())
            }
        }
        Ok(i64::from_ne_bytes(buf))
    }
}

impl AsRawFd for Ticker {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd.as_raw_fd()
    }
}
