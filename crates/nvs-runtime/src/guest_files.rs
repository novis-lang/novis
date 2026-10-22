//! The beneath walk (`rule:security/writes-open-beneath-a-handle`) for an extension's
//! `wasi:filesystem` alone.
//!
//! The WASI layer in `nvs-ext` has already resolved each path and checked it against the
//! instance's preopens, so nothing here checks a grant. It opens the resolved path one level at a
//! time, and a level that became a link after that check fails the call. Every `Core\IO` door
//! goes through [`crate::capability`] instead, which checks the grant first.

pub use crate::beneath::{
    Entry, Kind, Open, create_dir, entry_metadata, list, metadata, open_with, remove_dir,
    remove_file, rename,
};
