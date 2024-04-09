//! Ports of `ReadOnlyDir` and `ReadOnlyFile` to Preview 1 API.
//! Adapted from https://github.com/bytecodealliance/preview2-prototyping/pull/121

use std::{any::Any, path::PathBuf};

use tracing::{instrument, Level};
use wasi_common_preview1::{
    dir::{OpenResult, ReaddirCursor, ReaddirEntity},
    file::{Advice, FdFlags, FileType, Filestat, OFlags},
    Error, ErrorExt, SystemTimeSpec, WasiDir, WasiFile,
};

pub struct ReadOnlyDir(pub Box<dyn WasiDir>);

#[async_trait::async_trait]
impl WasiDir for ReadOnlyDir {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[instrument(name = "preview1_open_file", skip_all, level = Level::TRACE)]
    async fn open_file(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: OFlags,
        read: bool,
        write: bool,
        fdflags: FdFlags,
    ) -> Result<OpenResult, Error> {
        if write {
            Err(Error::perm())
        } else {
            let open_result = self
                .0
                .open_file(symlink_follow, path, oflags, read, write, fdflags)
                .await?;
            Ok(match open_result {
                OpenResult::File(f) => OpenResult::File(Box::new(ReadOnlyFile(f))),
                OpenResult::Dir(d) => OpenResult::Dir(Box::new(ReadOnlyDir(d))),
            })
        }
    }

    #[instrument(name = "preview1_create_dir", skip_all, level = Level::TRACE)]
    async fn create_dir(&self, _path: &str) -> Result<(), Error> {
        Err(Error::perm())
    }

    #[instrument(name = "preview1_readdir", skip_all, level = Level::TRACE)]
    async fn readdir(
        &self,
        cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<ReaddirEntity, Error>> + Send>, Error> {
        self.0.readdir(cursor).await
    }

    #[instrument(name = "preview1_symlink", skip_all, level = Level::TRACE)]
    async fn symlink(&self, _old_path: &str, _new_path: &str) -> Result<(), Error> {
        Err(Error::perm())
    }

    #[instrument(name = "preview1_remove_dir", skip_all, level = Level::TRACE)]
    async fn remove_dir(&self, _path: &str) -> Result<(), Error> {
        Err(Error::perm())
    }

    #[instrument(name = "preview1_unlink_file", skip_all, level = Level::TRACE)]
    async fn unlink_file(&self, _path: &str) -> Result<(), Error> {
        Err(Error::perm())
    }

    #[instrument(name = "preview1_read_link", skip_all, level = Level::TRACE)]
    async fn read_link(&self, path: &str) -> Result<PathBuf, Error> {
        self.0.read_link(path).await
    }

    #[instrument(name = "preview1_get_filestat", skip_all, level = Level::TRACE)]
    async fn get_filestat(&self) -> Result<Filestat, Error> {
        self.0.get_filestat().await
    }

    #[instrument(name = "preview1_get_path_filestat", skip_all, level = Level::TRACE)]
    async fn get_path_filestat(
        &self,
        path: &str,
        follow_symlinks: bool,
    ) -> Result<Filestat, Error> {
        self.0.get_path_filestat(path, follow_symlinks).await
    }

    #[instrument(name = "preview1_rename", skip_all, level = Level::TRACE)]
    async fn rename(
        &self,
        _path: &str,
        _dest_dir: &dyn WasiDir,
        _dest_path: &str,
    ) -> Result<(), Error> {
        Err(Error::perm())
    }

    #[instrument(name = "preview1_hard_link", skip_all, level = Level::TRACE)]
    async fn hard_link(
        &self,
        _path: &str,
        _target_dir: &dyn WasiDir,
        _target_path: &str,
    ) -> Result<(), Error> {
        Err(Error::perm())
    }

    #[instrument(name = "preview1_set_times", skip_all, level = Level::TRACE)]
    async fn set_times(
        &self,
        _path: &str,
        _atime: Option<SystemTimeSpec>,
        _mtime: Option<SystemTimeSpec>,
        _follow_symlinks: bool,
    ) -> Result<(), Error> {
        Err(Error::perm())
    }
}

pub struct ReadOnlyFile(pub Box<dyn WasiFile>);

#[async_trait::async_trait]
impl WasiFile for ReadOnlyFile {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[instrument(name = "preview1_get_filetype", skip_all, level = Level::TRACE)]
    async fn get_filetype(&self) -> Result<FileType, Error> {
        self.0.get_filetype().await
    }

    #[instrument(name = "preview1_pollable", skip_all, level = Level::TRACE)]
    #[cfg(unix)]
    fn pollable(&self) -> Option<rustix::fd::BorrowedFd> {
        self.0.pollable()
    }

    #[instrument(name = "preview1_pollable", skip_all, level = Level::TRACE)]
    #[cfg(windows)]
    fn pollable(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
        self.0.pollable()
    }

    #[instrument(name = "preview1_isatty", skip_all, level = Level::TRACE)]
    fn isatty(&self) -> bool {
        self.0.isatty()
    }

    #[instrument(name = "preview1_datasync", skip_all, level = Level::TRACE)]
    async fn datasync(&self) -> Result<(), Error> {
        self.0.datasync().await
    }

    #[instrument(name = "preview1_sync", skip_all, level = Level::TRACE)]
    async fn sync(&self) -> Result<(), Error> {
        self.0.sync().await
    }

    #[instrument(name = "preview1_get_fdflags", skip_all, level = Level::TRACE)]
    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        self.0.get_fdflags().await
    }

    #[instrument(name = "preview1_set_fdflags", skip_all, level = Level::TRACE)]
    async fn set_fdflags(&mut self, _flags: FdFlags) -> Result<(), Error> {
        Err(Error::perm())
    }

    #[instrument(name = "preview1_get_filestat", skip_all, level = Level::TRACE)]
    async fn get_filestat(&self) -> Result<Filestat, Error> {
        self.0.get_filestat().await
    }

    #[instrument(name = "preview1_set_filestat_size", skip_all, level = Level::TRACE)]
    async fn set_filestat_size(&self, _size: u64) -> Result<(), Error> {
        Err(Error::perm())
    }

    #[instrument(name = "preview1_advise", skip_all, level = Level::TRACE)]
    async fn advise(&self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
        self.0.advise(offset, len, advice).await
    }

    #[instrument(name = "preview1_set_times", skip_all, level = Level::TRACE)]
    async fn set_times(
        &self,
        _atime: Option<SystemTimeSpec>,
        _mtime: Option<SystemTimeSpec>,
    ) -> Result<(), Error> {
        Err(Error::perm())
    }

    #[instrument(name = "preview1_read_vectored", skip_all, level = Level::TRACE)]
    async fn read_vectored<'a>(&self, bufs: &mut [std::io::IoSliceMut<'a>]) -> Result<u64, Error> {
        self.0.read_vectored(bufs).await
    }

    #[instrument(name = "preview1_read_vectored_at", skip_all, level = Level::TRACE)]
    async fn read_vectored_at<'a>(
        &self,
        bufs: &mut [std::io::IoSliceMut<'a>],
        offset: u64,
    ) -> Result<u64, Error> {
        self.0.read_vectored_at(bufs, offset).await
    }

    #[instrument(name = "preview1_write_vectored_at", skip_all, level = Level::TRACE)]
    async fn write_vectored_at<'a>(
        &self,
        _bufs: &[std::io::IoSlice<'a>],
        _offset: u64,
    ) -> Result<u64, Error> {
        Err(Error::perm())
    }

    #[instrument(name = "preview1_seek", skip_all, level = Level::TRACE)]
    async fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error> {
        self.0.seek(pos).await
    }

    #[instrument(name = "preview1_peek", skip_all, level = Level::TRACE)]
    async fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
        self.0.peek(buf).await
    }

    #[instrument(name = "preview1_num_ready_bytes", skip_all, level = Level::TRACE)]
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        self.0.num_ready_bytes()
    }

    #[instrument(name = "preview1_readable", skip_all, level = Level::TRACE)]
    async fn readable(&self) -> Result<(), Error> {
        self.0.readable().await
    }

    #[instrument(name = "preview1_writable", skip_all, level = Level::TRACE)]
    async fn writable(&self) -> Result<(), Error> {
        Err(Error::perm())
    }
}
