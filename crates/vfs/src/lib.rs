//! The board's vfs partition, LittleFS v2, opened by the recovery image's own app_fs.c (the app_fs component of
//! this crate). Mounted once, for as long as the program runs (`app_fs_keep_mounted`): a mount walks every directory
//! of the tree, and one per call held the network's loop for seconds on a /journal (a device in service, 26.09). Every call
//! takes app_fs.c's lock, opens, does one thing and closes: no file stays open between calls, and app_fs.c lets one
//! call in at a time.

use std::ffi::CString;

use esp_idf_svc::sys::app_fs as c;

/// A LittleFS error code, or -1 when the partition could not be mounted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(pub i32);

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            -1 => write!(f, "vfs not mounted"),
            -2 => write!(f, "no such file"),
            code => write!(f, "littlefs error {code}"),
        }
    }
}

struct Mounted(*mut c::lfs2_t);

impl Mounted {
    fn new() -> Result<Self, Error> {
        let fs = unsafe { c::app_fs_begin() };
        if fs.is_null() {
            Err(Error(-1))
        } else {
            Ok(Mounted(fs))
        }
    }
}

impl Drop for Mounted {
    fn drop(&mut self) {
        unsafe { c::app_fs_end() };
    }
}

/// Mounts vfs for good: the first call creates app_fs.c's lock, so main makes it before any thread can.
pub fn open_once() -> Result<(), Error> {
    unsafe { c::app_fs_keep_mounted() };
    Mounted::new().map(|_| ())
}

/// What the tree says of a file without reading it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    pub size: usize,
    /// The block its last piece lies in; None for a file small enough to lie inside its directory. A file written
    /// again lands in new blocks - LittleFS copies on write -, so the block changes with the bytes.
    pub head: Option<u32>,
}

/// The size of the file at PATH and where it lies, from its directory entry: one open, not a read of the file.
pub fn stat(path: &str) -> Result<Stat, Error> {
    let fs = Mounted::new()?;
    let path = CString::new(path).map_err(|_| Error(c::lfs2_error_LFS2_ERR_INVAL))?;
    let mut file: c::lfs2_file_t = unsafe { core::mem::zeroed() };
    let err = unsafe {
        c::lfs2_file_opencfg(fs.0, &mut file, path.as_ptr(), c::lfs2_open_flags_LFS2_O_RDONLY as i32, c::app_fs_file_config(0))
    };
    if err < 0 {
        return Err(Error(err));
    }
    let size = unsafe { c::lfs2_file_size(fs.0, &mut file) };
    // LFS2_BLOCK_INLINE, private to lfs2.c: the bytes lie in the directory's own block
    let head = (file.ctz.head != u32::MAX - 1).then_some(file.ctz.head);
    unsafe { c::lfs2_file_close(fs.0, &mut file) };
    match size {
        e if e < 0 => Err(Error(e)),
        size => Ok(Stat { size: size as usize, head }),
    }
}

/// Reads PATH piece by piece into SINK; the size read. A sink that fails stops the read.
pub fn read<E>(path: &str, mut sink: impl FnMut(&[u8]) -> Result<(), E>) -> Result<Result<usize, E>, Error> {
    let fs = Mounted::new()?;
    let path = CString::new(path).map_err(|_| Error(c::lfs2_error_LFS2_ERR_INVAL))?;
    let mut file: c::lfs2_file_t = unsafe { core::mem::zeroed() };
    let err = unsafe {
        c::lfs2_file_opencfg(fs.0, &mut file, path.as_ptr(), c::lfs2_open_flags_LFS2_O_RDONLY as i32, c::app_fs_file_config(0))
    };
    if err < 0 {
        return Err(Error(err));
    }
    let mut buf = [0u8; 1024];
    let mut total = 0;
    let out = loop {
        let n = unsafe { c::lfs2_file_read(fs.0, &mut file, buf.as_mut_ptr().cast(), buf.len() as _) };
        if n < 0 {
            unsafe { c::lfs2_file_close(fs.0, &mut file) };
            return Err(Error(n));
        }
        if n == 0 {
            break Ok(total);
        }
        if let Err(e) = sink(&buf[..n as usize]) {
            break Err(e);
        }
        total += n as usize;
    };
    unsafe { c::lfs2_file_close(fs.0, &mut file) };
    Ok(out)
}

/// Reads from PATH at AT into BUF; the count read, 0 at its end. A piece at a time, for a reader that sends each piece
/// before it asks for the next - the network's loop, which may not hold a callback across a write that waits.
pub fn read_at(path: &str, at: usize, buf: &mut [u8]) -> Result<usize, Error> {
    let fs = Mounted::new()?;
    let path = CString::new(path).map_err(|_| Error(c::lfs2_error_LFS2_ERR_INVAL))?;
    let mut file: c::lfs2_file_t = unsafe { core::mem::zeroed() };
    let err = unsafe {
        c::lfs2_file_opencfg(fs.0, &mut file, path.as_ptr(), c::lfs2_open_flags_LFS2_O_RDONLY as i32, c::app_fs_file_config(0))
    };
    if err < 0 {
        return Err(Error(err));
    }
    let sought = unsafe { c::lfs2_file_seek(fs.0, &mut file, at as _, c::lfs2_whence_flags_LFS2_SEEK_SET as i32) };
    let n = match sought {
        e if e < 0 => e,
        _ => unsafe { c::lfs2_file_read(fs.0, &mut file, buf.as_mut_ptr().cast(), buf.len() as _) },
    };
    unsafe { c::lfs2_file_close(fs.0, &mut file) };
    match n {
        e if e < 0 => Err(Error(e)),
        n => Ok(n as usize),
    }
}

/// Writes DATA as PATH, making the folders above it; replaces a file that was there.
pub fn write(path: &str, data: &[u8]) -> Result<(), Error> {
    let fs = Mounted::new()?;
    let cpath = CString::new(path).map_err(|_| Error(c::lfs2_error_LFS2_ERR_INVAL))?;
    let err = unsafe { c::app_fs_make_parents(fs.0, cpath.as_ptr()) };
    if err < 0 {
        return Err(Error(err));
    }
    let mut file: c::lfs2_file_t = unsafe { core::mem::zeroed() };
    let flags = c::lfs2_open_flags_LFS2_O_WRONLY | c::lfs2_open_flags_LFS2_O_CREAT | c::lfs2_open_flags_LFS2_O_TRUNC;
    let err = unsafe { c::lfs2_file_opencfg(fs.0, &mut file, cpath.as_ptr(), flags as i32, c::app_fs_file_config(0)) };
    if err < 0 {
        return Err(Error(err));
    }
    let n = unsafe { c::lfs2_file_write(fs.0, &mut file, data.as_ptr().cast(), data.len() as _) };
    let closed = unsafe { c::lfs2_file_close(fs.0, &mut file) };
    if n < 0 {
        return Err(Error(n));
    }
    if closed < 0 {
        return Err(Error(closed));
    }
    Ok(())
}

/// Adds DATA to the end of PATH, making it and the folders above it when they are not there. LittleFS keeps the
/// file as it was until the close: a power cut in the middle loses the new bytes, never the old ones.
pub fn append(path: &str, data: &[u8]) -> Result<(), Error> {
    let fs = Mounted::new()?;
    let cpath = CString::new(path).map_err(|_| Error(c::lfs2_error_LFS2_ERR_INVAL))?;
    let err = unsafe { c::app_fs_make_parents(fs.0, cpath.as_ptr()) };
    if err < 0 {
        return Err(Error(err));
    }
    let mut file: c::lfs2_file_t = unsafe { core::mem::zeroed() };
    let flags = c::lfs2_open_flags_LFS2_O_WRONLY | c::lfs2_open_flags_LFS2_O_CREAT | c::lfs2_open_flags_LFS2_O_APPEND;
    let err = unsafe { c::lfs2_file_opencfg(fs.0, &mut file, cpath.as_ptr(), flags as i32, c::app_fs_file_config(0)) };
    if err < 0 {
        return Err(Error(err));
    }
    let n = unsafe { c::lfs2_file_write(fs.0, &mut file, data.as_ptr().cast(), data.len() as _) };
    let closed = unsafe { c::lfs2_file_close(fs.0, &mut file) };
    if n < 0 {
        return Err(Error(n));
    }
    if closed < 0 {
        return Err(Error(closed));
    }
    Ok(())
}

/// Removes the file PATH.
pub fn remove(path: &str) -> Result<(), Error> {
    let fs = Mounted::new()?;
    let cpath = CString::new(path).map_err(|_| Error(c::lfs2_error_LFS2_ERR_INVAL))?;
    let err = unsafe { c::lfs2_remove(fs.0, cpath.as_ptr()) };
    if err < 0 {
        return Err(Error(err));
    }
    Ok(())
}

/// The names of the files right in DIR, without its folders; none when DIR is not there.
pub fn files(dir: &str) -> Result<Vec<String>, Error> {
    let fs = Mounted::new()?;
    let cdir = CString::new(dir).map_err(|_| Error(c::lfs2_error_LFS2_ERR_INVAL))?;
    let mut d: c::lfs2_dir_t = unsafe { core::mem::zeroed() };
    let err = unsafe { c::lfs2_dir_open(fs.0, &mut d, cdir.as_ptr()) };
    if err == c::lfs2_error_LFS2_ERR_NOENT {
        return Ok(Vec::new());
    }
    if err < 0 {
        return Err(Error(err));
    }
    let mut names = Vec::new();
    let mut info: c::lfs2_info = unsafe { core::mem::zeroed() };
    let out = loop {
        let n = unsafe { c::lfs2_dir_read(fs.0, &mut d, &mut info) };
        if n < 0 {
            break Err(Error(n));
        }
        if n == 0 {
            break Ok(());
        }
        if u32::from(info.type_) == c::lfs2_type_LFS2_TYPE_REG {
            let name = unsafe { core::ffi::CStr::from_ptr(info.name.as_ptr()) };
            names.push(name.to_string_lossy().into_owned());
        }
    };
    unsafe { c::lfs2_dir_close(fs.0, &mut d) };
    out.map(|()| names)
}

/// Blocks in use and in all, for a person reading the board's state.
pub fn usage() -> Result<(u32, u32), Error> {
    let fs = Mounted::new()?;
    let mut usage: c::app_fs_usage_t = unsafe { core::mem::zeroed() };
    if !unsafe { c::app_fs_usage(fs.0, &mut usage) } {
        return Err(Error(-1));
    }
    Ok((usage.used_blocks as u32, usage.blocks as u32))
}
