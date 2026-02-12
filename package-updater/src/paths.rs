use std::path::PathBuf;

/// Get the XDG runtime directory, falling back to /tmp
pub fn runtime_dir() -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        eprintln!("Warning: XDG_RUNTIME_DIR not set, using /tmp");
        "/tmp".to_string()
    });
    PathBuf::from(dir)
}

/// Path to the lock file for preventing concurrent update checks
pub fn lock_path() -> PathBuf {
    runtime_dir().join("cosmic-package-updater.lock")
}

/// Path to the sync file for notifying other instances
pub fn sync_path() -> PathBuf {
    runtime_dir().join("cosmic-package-updater.sync")
}
