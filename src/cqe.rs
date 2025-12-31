use crate::io_uring_cqe;
use rustix::io::Errno;

pub const IORING_CQE_F_BUFFER: u32 = 1 << 0;
pub const IORING_CQE_F_MORE: u32 = 1 << 1;
pub const IORING_CQE_F_SOCK_NONEMPTY: u32 = 1 << 2;
pub const IORING_CQE_F_TIMEOUT: u32 = 1 << 3;
pub const IORING_CQE_F_NOTIFICATION: u32 = 1 << 4;

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CqeFlags {
    Buffer = IORING_CQE_F_BUFFER,
    More = IORING_CQE_F_MORE,
    SockNonempty = IORING_CQE_F_SOCK_NONEMPTY,
    Timeout = IORING_CQE_F_TIMEOUT,
    Notification = IORING_CQE_F_NOTIFICATION,
}

impl CqeFlags {
    #[must_use]
    pub fn bits(self) -> u32 {
        self as u32
    }

    #[must_use]
    pub fn from_bits(bits: u32) -> Option<Self> {
        match bits {
            IORING_CQE_F_BUFFER => Some(Self::Buffer),
            IORING_CQE_F_MORE => Some(Self::More),
            IORING_CQE_F_SOCK_NONEMPTY => Some(Self::SockNonempty),
            IORING_CQE_F_TIMEOUT => Some(Self::Timeout),
            IORING_CQE_F_NOTIFICATION => Some(Self::Notification),
            _ => None,
        }
    }
}

#[must_use]
pub fn cqe_result(cqe: &io_uring_cqe) -> i32 {
    cqe.res
}

#[must_use]
pub fn cqe_user_data(cqe: &io_uring_cqe) -> u64 {
    cqe.user_data
}

#[must_use]
pub fn cqe_flags(cqe: &io_uring_cqe) -> u32 {
    cqe.flags
}

#[must_use]
pub fn cqe_res_to_result(res: i32) -> Result<i32, Errno> {
    if res >= 0 {
        Ok(res)
    } else {
        Err(Errno::from_raw_os_error(-res))
    }
}

#[must_use]
pub fn cqe_result_to_result(cqe: &io_uring_cqe) -> Result<i32, Errno> {
    cqe_res_to_result(cqe.res)
}
