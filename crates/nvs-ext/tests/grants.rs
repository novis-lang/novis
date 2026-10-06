//! `rule:security/extension-grants-are-an-intersection`: what a guest may reach is what its entry,
//! its manifest and its caller all hold, and each of the three narrows it on its own.
//!
//! The canonicaliser is a fake with a fixed set of folders and one link, so a case states exactly
//! which paths exist and nothing depends on the machine running it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nvs_config::Setting;
use nvs_config::capability::Cap;
use nvs_config::extension::Granted;
use nvs_config::resolve::Files;
use nvs_config::tree::{CapFs, CapNet, Capabilities};
use nvs_config::trust::Untrusted;
use nvs_ext::grants::{Caller, Effective, effective};
use nvs_ext::manifest::Requests;

/// Folders that exist, each mapped to its canonical path: itself, or a link's target.
struct Fake(BTreeMap<PathBuf, PathBuf>);

fn fake() -> Fake {
    let mut known = BTreeMap::new();
    for dir in [
        "/srv",
        "/srv/data",
        "/srv/data/geo",
        "/srv/data2",
        "/srv/logs",
        "/srv/out",
    ] {
        known.insert(PathBuf::from(dir), PathBuf::from(dir));
    }
    known.insert(PathBuf::from("/srv/alias"), PathBuf::from("/srv/data/geo"));
    Fake(known)
}

impl Files for Fake {
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted> {
        self.canonical(path).map_err(Untrusted::Unreadable)
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        self.0
            .get(path)
            .cloned()
            .ok_or_else(|| "no such folder".to_string())
    }

    fn read(&self, _path: &Path) -> Result<String, String> {
        Err("not a file".to_string())
    }

    fn read_bytes(&self, _path: &Path) -> Result<Vec<u8>, String> {
        Err("not a file".to_string())
    }

    fn exposure(&self, _path: &Path) -> Option<String> {
        None
    }

    fn list(&self, _dir: &Path) -> Result<Vec<PathBuf>, String> {
        Ok(Vec::new())
    }

    fn exists(&self, path: &Path) -> bool {
        self.0.contains_key(path)
    }
}

fn paths(written: &[&str]) -> Vec<PathBuf> {
    written.iter().map(PathBuf::from).collect()
}

fn strings(written: &[&str]) -> Vec<String> {
    written.iter().map(ToString::to_string).collect()
}

fn list(written: &[&str]) -> Option<Setting> {
    Some(Setting::List(strings(written)))
}

/// The entry every case grants, unless it says otherwise.
fn entry() -> Granted {
    Granted {
        read: paths(&["/srv/data", "/srv/logs", "/srv/alias"]),
        write: paths(&["/srv/out"]),
        connect: strings(&["tiles.example.com", "api.example.com"]),
    }
}

/// A manifest asking for every kind, and for one of the entry's two hosts.
fn requests() -> Requests {
    Requests {
        read: strings(&["data/geo/"]),
        write: strings(&["out/"]),
        connect: strings(&["Tiles.Example.com"]),
    }
}

/// A caller holding every kind: all of `/srv` and both hosts.
fn everything() -> Capabilities {
    Capabilities {
        fs: Some(CapFs {
            read: Some(Setting::Bool(true)),
            write: Some(Setting::Bool(true)),
        }),
        net: Some(CapNet {
            connect: list(&["tiles.example.com", "api.example.com"]),
            ..CapNet::default()
        }),
        ..Capabilities::default()
    }
}

fn caller(caps: &Capabilities) -> Caller<'_> {
    Caller {
        capabilities: Some(caps),
        narrowed: None,
    }
}

#[test]
fn the_effective_set_is_the_intersection_of_entry_manifest_and_caller() {
    let caps = Capabilities {
        fs: Some(CapFs {
            read: list(&["/srv/data/geo", "/srv/out"]),
            write: list(&["/srv/out"]),
        }),
        ..everything()
    };
    let got = effective(&entry(), &requests(), caller(&caps), &fake());
    // `/srv/data` narrows to the caller's `/srv/data/geo`, and `/srv/alias` resolves to the same
    // folder. The caller holds nothing under `/srv/logs`, so it is not kept.
    assert_eq!(
        got,
        Effective {
            read: paths(&["/srv/data/geo"]),
            write: paths(&["/srv/out"]),
            connect: strings(&["tiles.example.com"]),
        }
    );

    let all = effective(&entry(), &requests(), caller(&everything()), &fake());
    assert_eq!(
        all.read,
        paths(&["/srv/data", "/srv/data/geo", "/srv/logs"]),
        "a caller holding every folder keeps each entry root, canonical"
    );
}

#[test]
fn an_entry_with_no_grants_holds_no_io_whatever_the_manifest_requests() {
    let caps = everything();
    let got = effective(&Granted::default(), &requests(), caller(&caps), &fake());
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn a_manifest_requesting_nothing_holds_no_io_whatever_the_entry_grants() {
    let caps = everything();
    let got = effective(&entry(), &Requests::default(), caller(&caps), &fake());
    assert!(got.is_empty(), "{got:?}");

    let reads_only = Requests {
        read: strings(&["data/"]),
        ..Requests::default()
    };
    let got = effective(&entry(), &reads_only, caller(&caps), &fake());
    assert!(!got.read.is_empty());
    assert!(
        got.write.is_empty(),
        "a manifest that writes nothing gets no write root"
    );
    assert!(
        got.connect.is_empty(),
        "a manifest that names no host gets none"
    );
}

#[test]
fn a_caller_without_the_grant_narrows_the_extension() {
    let none = effective(&entry(), &requests(), Caller::default(), &fake());
    assert!(
        none.is_empty(),
        "a caller with no capabilities holds nothing: {none:?}"
    );

    // A sibling whose name starts with the root's is not inside it.
    let sibling = Capabilities {
        fs: Some(CapFs {
            read: list(&["/srv/data2"]),
            write: None,
        }),
        net: None,
        ..Capabilities::default()
    };
    let got = effective(&entry(), &requests(), caller(&sibling), &fake());
    assert!(got.is_empty(), "{got:?}");

    // A write root reads too, so a caller that may only write keeps no write root.
    let writes_only = Capabilities {
        fs: Some(CapFs {
            read: None,
            write: Some(Setting::Bool(true)),
        }),
        ..everything()
    };
    let got = effective(&entry(), &requests(), caller(&writes_only), &fake());
    assert!(got.read.is_empty() && got.write.is_empty(), "{got:?}");
    assert_eq!(got.connect, strings(&["tiles.example.com"]));
}

#[test]
fn an_isolate_narrowed_caller_narrows_the_extension() {
    let caps = everything();
    let only_hosts = [Cap::NetConnect];
    let got = effective(
        &entry(),
        &requests(),
        Caller {
            capabilities: Some(&caps),
            narrowed: Some(&only_hosts),
        },
        &fake(),
    );
    assert!(got.read.is_empty() && got.write.is_empty(), "{got:?}");
    assert_eq!(got.connect, strings(&["tiles.example.com"]));

    let only_reads = [Cap::FsRead];
    let got = effective(
        &entry(),
        &requests(),
        Caller {
            capabilities: Some(&caps),
            narrowed: Some(&only_reads),
        },
        &fake(),
    );
    assert!(!got.read.is_empty());
    assert!(got.write.is_empty() && got.connect.is_empty(), "{got:?}");
}
