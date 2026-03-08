use crate::transaction_tracker::TransactionTracker;
use crate::vfs::NFSFileSystem;
use std::fmt;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct RPCContext<VFS: NFSFileSystem> {
    pub local_port: u16,
    pub client_addr: String,
    pub auth: crate::rpc::auth_unix,
    pub vfs: Arc<VFS>,
    pub mount_signal: Option<mpsc::Sender<bool>>,
    pub export_name: Arc<String>,
    pub transaction_tracker: Arc<TransactionTracker>,
}

impl<VFS: NFSFileSystem> Clone for RPCContext<VFS> {
    fn clone(&self) -> Self {
        RPCContext {
            local_port: self.local_port,
            client_addr: self.client_addr.clone(),
            auth: self.auth.clone(),
            vfs: self.vfs.clone(),
            mount_signal: self.mount_signal.clone(),
            export_name: self.export_name.clone(),
            transaction_tracker: self.transaction_tracker.clone(),
        }
    }
}

impl<VFS: NFSFileSystem> fmt::Debug for RPCContext<VFS> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RPCContext")
            .field("local_port", &self.local_port)
            .field("client_addr", &self.client_addr)
            .field("auth", &self.auth)
            .finish()
    }
}
