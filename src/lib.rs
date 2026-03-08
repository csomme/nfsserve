#![cfg_attr(feature = "strict", deny(warnings))]
// num_derive's FromPrimitive/ToPrimitive macros generate non-local impls; suppress
// until num_derive is updated.
#![allow(non_local_definitions)]

mod context;
mod rpc;
mod rpcwire;
mod write_counter;
pub mod xdr;

mod mount;
mod mount_handlers;

mod portmap;
mod portmap_handlers;

pub mod nfs;
mod nfs_handlers;

#[cfg(not(target_os = "windows"))]
pub mod fs_util;

pub mod tcp;
mod transaction_tracker;
pub mod vfs;
