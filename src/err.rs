pub enum InitError {
    UnsupportedKernel,
    MmapFailed(rustix::io::Errno),
    FcntlFailed(rustix::io::Errno),
    SyscallFailed(rustix::io::Errno),
    RegisterFailed(rustix::io::Errno),
    InvalidParameters,
    FeatureNotSupported(u32),
}

pub enum EnterError {
    SyscallFailed(rustix::io::Errno),
    BadOffset,
}

pub type IoUringResult<T> = Result<T, InitError>;

impl core::fmt::Debug for InitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedKernel => write!(f, "UnsupportedKernel"),
            Self::MmapFailed(e) => write!(f, "MmapFailed({e})"),
            Self::FcntlFailed(e) => write!(f, "FcntlFailed({e})"),
            Self::SyscallFailed(e) => write!(f, "SyscallFailed({e})"),
            Self::RegisterFailed(e) => write!(f, "RegisterFailed({e})"),
            Self::InvalidParameters => write!(f, "InvalidParameters"),
            Self::FeatureNotSupported(feat) => write!(f, "FeatureNotSupported({feat:#x})"),
        }
    }
}

impl core::fmt::Debug for EnterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SyscallFailed(e) => write!(f, "SyscallFailed({e})"),
            Self::BadOffset => write!(f, "BadOffset"),
        }
    }
}

impl From<rustix::io::Errno> for EnterError {
    fn from(e: rustix::io::Errno) -> Self {
        Self::SyscallFailed(e)
    }
}
