use crate::nfs::*;
use crate::nfs;
use async_trait::async_trait;
use std::cmp::Ordering;
use std::sync::Once;
use std::time::SystemTime;
use std::fmt::{Debug, Display};
use std::convert::TryFrom;
use crate::mount::fhandle3;
use crate::nfs_handlers::stable_how;

#[derive(Default, Debug)]
pub struct DirEntrySimple {
    pub fileid: fileid3,
    pub name: filename3,
}

#[derive(Default, Debug)]
pub struct ReadDirSimpleResult {
    pub entries: Vec<DirEntrySimple>,
    pub end: bool,
}

// Generic directory entry that uses custom file handle type
#[derive(Debug)]
pub struct DirEntry<H> {
    pub handle: H,
    pub name: filename3,
    pub attr: fattr3,
}

// Generic readdir result that uses custom file handle type
#[derive(Debug)]
pub struct ReadDirResult<H> {
    pub entries: Vec<DirEntry<H>>,
    pub end: bool,
}

impl<H: Clone + Into<fileid3>> ReadDirResult<H> {
    pub fn to_simple(&self) -> ReadDirSimpleResult {
        let entries: Vec<DirEntrySimple> = self
            .entries
            .iter()
            .map(|e| DirEntrySimple {
                fileid: e.handle.clone().into(),
                name: e.name.clone(),
            })
            .collect();
        ReadDirSimpleResult {
            entries,
            end: self.end,
        }
    }
}

static mut GENERATION_NUMBER: u64 = 0;
static GENERATION_NUMBER_INIT: Once = Once::new();

pub fn get_generation_number() -> u64 {
    unsafe {
        GENERATION_NUMBER_INIT.call_once(|| {
            GENERATION_NUMBER = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
        });
        GENERATION_NUMBER
    }
}

/// What capabilities are supported
pub enum VFSCapabilities {
    ReadOnly,
    ReadWrite,
}

/// The basic API to implement to provide an NFS file system
///
/// This trait uses a generic FileHandle type defined by the implementer
#[async_trait]
pub trait NFSFileSystem: Send + Sync {
    /// The type used for file handles within this implementation
    type FileHandle: Clone + Send + Sync + Debug + 'static
    + Into<nfs_fh3>
    + Into<Vec<u8>>
    + for<'a> TryFrom<&'a nfs_fh3, Error = nfsstat3>
    + Into<u64>;

    /// Returns the set of capabilities supported
    fn capabilities(&self) -> VFSCapabilities;

    /// Returns the handle of the root directory "/"
    fn root_dir(&self) -> Self::FileHandle;

    /// Look up the handle of a path in a directory
    async fn lookup(&self, dir_handle: &Self::FileHandle, filename: &filename3)
                    -> Result<Self::FileHandle, nfsstat3>;

    /// Returns the attributes of a file
    async fn getattr(&self, handle: &Self::FileHandle) -> Result<fattr3, nfsstat3>;

    /// Sets the attributes of a file
    /// Returns Err(nfsstat3::NFS3ERR_ROFS) if readonly
    async fn setattr(&self, handle: &Self::FileHandle, setattr: sattr3) -> Result<fattr3, nfsstat3>;

    /// Reads the contents of a file returning (bytes, EOF)
    async fn read(&self, handle: &Self::FileHandle, offset: u64, count: u32)
                  -> Result<(Vec<u8>, bool), nfsstat3>;

    /// Writes the contents of a file
    /// Returns Err(nfsstat3::NFS3ERR_ROFS) if readonly
    async fn write(&self, handle: &Self::FileHandle, offset: u64, data: &[u8])
                   -> Result<fattr3, nfsstat3>;

    /// Creates a file with the specified attributes
    /// Returns Err(nfsstat3::NFS3ERR_ROFS) if readonly
    async fn create(
        &self,
        dir_handle: &Self::FileHandle,
        filename: &filename3,
        attr: sattr3,
    ) -> Result<(Self::FileHandle, fattr3), nfsstat3>;

    /// Creates a file if it does not already exist
    /// Returns Err(nfsstat3::NFS3ERR_ROFS) if readonly
    async fn create_exclusive(
        &self,
        dir_handle: &Self::FileHandle,
        filename: &filename3,
    ) -> Result<Self::FileHandle, nfsstat3>;

    /// Makes a directory with the following attributes
    /// Returns Err(nfsstat3::NFS3ERR_ROFS) if readonly
    async fn mkdir(
        &self,
        dir_handle: &Self::FileHandle,
        dirname: &filename3,
    ) -> Result<(Self::FileHandle, fattr3), nfsstat3>;

    /// Removes a file
    /// Returns Err(nfsstat3::NFS3ERR_ROFS) if readonly
    async fn remove(&self, dir_handle: &Self::FileHandle, filename: &filename3)
                    -> Result<(), nfsstat3>;

    /// Renames a file
    /// Returns Err(nfsstat3::NFS3ERR_ROFS) if readonly
    async fn rename(
        &self,
        from_dir_handle: &Self::FileHandle,
        from_filename: &filename3,
        to_dir_handle: &Self::FileHandle,
        to_filename: &filename3,
    ) -> Result<(), nfsstat3>;

    /// Returns the contents of a directory with pagination
    async fn readdir(
        &self,
        dir_handle: &Self::FileHandle,
        start_after: fileid3,
        max_entries: usize,
    ) -> Result<ReadDirResult<Self::FileHandle>, nfsstat3>;

    /// Simple version of readdir
    /// Only need to return filename and id
    async fn readdir_simple(
        &self,
        dir_handle: &Self::FileHandle,
        count: usize,
    ) -> Result<ReadDirSimpleResult, nfsstat3> {
        let result = self.readdir(dir_handle, 0, count).await?;
        Ok(result.to_simple())
    }

    /// Makes a symlink with the specified attributes
    /// Returns Err(nfsstat3::NFS3ERR_ROFS) if readonly
    async fn symlink(
        &self,
        dir_handle: &Self::FileHandle,
        linkname: &filename3,
        symlink: &nfspath3,
        attr: &sattr3,
    ) -> Result<(Self::FileHandle, fattr3), nfsstat3>;

    /// Reads a symlink
    async fn readlink(&self, handle: &Self::FileHandle) -> Result<nfspath3, nfsstat3>;

    /// Write with stability level
    async fn write_with_stability(
        &self,
        handle: &Self::FileHandle,
        offset: nfs::offset3,
        data: &[u8],
        _stability: stable_how,
    ) -> Result<(nfs::fattr3, stable_how), nfs::nfsstat3> {
        Ok((self.write(handle, offset, data).await?, stable_how::FILE_SYNC))
    }

    /// Commit pending writes
    async fn commit(
        &self,
        _handle: &Self::FileHandle,
        _offset: nfs::offset3,
        _count: nfs::count3,
    ) -> Result<nfs::fattr3, nfs::nfsstat3> {
        Err(nfsstat3::NFS3ERR_NOTSUPP)
    }

    /// Get static file system Information
    async fn fsinfo(
        &self,
        root_handle: &Self::FileHandle,
    ) -> Result<fsinfo3, nfsstat3> {
        let dir_attr: nfs::post_op_attr = match self.getattr(root_handle).await {
            Ok(v) => nfs::post_op_attr::attributes(v),
            Err(_) => nfs::post_op_attr::Void,
        };

        let res = fsinfo3 {
            obj_attributes: dir_attr,
            rtmax: 1024 * 1024,
            rtpref: 1024 * 124,
            rtmult: 1024 * 1024,
            wtmax: 1024 * 1024,
            wtpref: 1024 * 1024,
            wtmult: 1024 * 1024,
            dtpref: 1024 * 1024,
            maxfilesize: 128 * 1024 * 1024 * 1024,
            time_delta: nfs::nfstime3 {
                seconds: 0,
                nseconds: 1000000,
            },
            properties: nfs::FSF_SYMLINK | nfs::FSF_HOMOGENEOUS | nfs::FSF_CANSETTIME,
        };
        Ok(res)
    }

    /// Converts a complete path to a file handle
    async fn path_to_handle(&self, path: &[u8]) -> Result<Self::FileHandle, nfsstat3> {
        let splits = path.split(|&r| r == b'/');
        let mut handle = self.root_dir();
        for component in splits {
            if component.is_empty() {
                continue;
            }
            handle = self.lookup(&handle, &component.into()).await?;
        }
        Ok(handle)
    }

    /// Returns a unique server ID
    fn serverid(&self) -> cookieverf3 {
        let gennum = get_generation_number();
        gennum.to_le_bytes()
    }
}
