//! `rule:packaging/the-windows-binary-says-what-it-is`, read back out of the
//! linked binary: `build/winres.rs` writes the resource by hand, so what holds
//! it is a reader that shares none of that code walking the `.exe` the linker
//! produced. The icon is compared image for image with the committed `.ico`,
//! and the version strings with the manifest and `LICENSE` they are taken from.
//!
//! A bundle `nvs build --compile` writes is read the same way: with a
//! `package.toml` it carries the program's name, version and icon, and without
//! one it carries the host's.

#![cfg(all(windows, target_env = "msvc"))]

use std::path::{Path, PathBuf};
use std::process::Command;

use object::LittleEndian as LE;
use object::read::pe::{
    PeFile64, ResourceDirectory, ResourceDirectoryEntryData, ResourceDirectoryTable,
    ResourceNameOrId,
};

const RT_ICON: u16 = 3;
const RT_MESSAGETABLE: u16 = 11;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;

/// The icon `build.rs` picks for the profile this test is built in.
const ICON: &[u8] = if cfg!(debug_assertions) {
    include_bytes!("../assets/nvs-debug.ico")
} else {
    include_bytes!("../assets/nvs.ico")
};

/// The icon of the other profile, which a bundle's `package.toml` names, so it
/// differs from the host's.
const OTHER_ICON: &[u8] = if cfg!(debug_assertions) {
    include_bytes!("../assets/nvs.ico")
} else {
    include_bytes!("../assets/nvs-debug.ico")
};

const LICENSE: &str = include_str!("../../../LICENSE");

/// Every resource in the built `nvs.exe`, as its numeric type and its data.
fn resources() -> Vec<(u16, Vec<u8>)> {
    resources_of(Path::new(env!("CARGO_BIN_EXE_nvs")))
}

/// Every resource in the executable at `path`, as its numeric type and its data.
fn resources_of(path: &Path) -> Vec<(u16, Vec<u8>)> {
    let exe = std::fs::read(path).expect("the built binary is readable");
    let file = PeFile64::parse(&*exe).expect("the built binary is a 64-bit PE file");
    let sections = file.section_table();
    let directory = file
        .data_directories()
        .resource_directory(&*exe, &sections)
        .expect("the resource directory parses")
        .expect("the binary has a resource directory");

    let mut found = Vec::new();
    let root = directory.root().expect("the root table parses");
    for entry in root.entries {
        let ResourceNameOrId::Id(kind) = entry.name_or_id() else {
            continue;
        };
        let data = entry.data(directory).expect("a type's entry parses");
        let mut leaves = Vec::new();
        collect(directory, data, &mut leaves);
        for leaf in leaves {
            let bytes = sections
                .pe_data_at(&*exe, leaf.0)
                .and_then(|data| data.get(..leaf.1))
                .expect("a resource's data is inside a section");
            found.push((kind, bytes.to_vec()));
        }
    }
    found
}

/// The address and size of every data entry under `data`, through the name
/// and language levels.
fn collect(
    directory: ResourceDirectory<'_>,
    data: ResourceDirectoryEntryData<'_>,
    out: &mut Vec<(u32, usize)>,
) {
    match data {
        ResourceDirectoryEntryData::Data(entry) => out.push((
            entry.offset_to_data.get(LE),
            usize::try_from(entry.size.get(LE)).expect("a size fits"),
        )),
        ResourceDirectoryEntryData::Table(ResourceDirectoryTable { entries, .. }) => {
            for entry in entries {
                let data = entry.data(directory).expect("a nested entry parses");
                collect(directory, data, out);
            }
        }
    }
}

fn of_kind(kind: u16) -> Vec<Vec<u8>> {
    kind_in(&resources(), kind)
}

fn kind_in(resources: &[(u16, Vec<u8>)], kind: u16) -> Vec<Vec<u8>> {
    resources
        .iter()
        .filter(|(found, _)| *found == kind)
        .map(|(_, data)| data.clone())
        .collect()
}

fn u16_at(bytes: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]))
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    let field = [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]];
    usize::try_from(u32::from_le_bytes(field)).expect("a field fits")
}

/// Every text value in the version block at `at`, by its key.
///
/// A node is its length, its value's length, whether the value is text, a
/// NUL-terminated UTF-16 key, the value and then child nodes, the last three
/// each on a 32-bit boundary.
fn strings(block: &[u8], at: usize, out: &mut Vec<(String, String)>) {
    let end = at + u16_at(block, at);
    let value_len = u16_at(block, at + 2);
    let text = u16_at(block, at + 4) == 1;

    let mut cursor = at + 6;
    let mut key = Vec::new();
    loop {
        let unit = u16::from_le_bytes([block[cursor], block[cursor + 1]]);
        cursor += 2;
        if unit == 0 {
            break;
        }
        key.push(unit);
    }
    cursor = cursor.next_multiple_of(4);

    if text && value_len > 0 {
        let value: Vec<u16> = block[cursor..cursor + value_len * 2]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|unit| *unit != 0)
            .collect();
        out.push((
            String::from_utf16(&key).expect("a key is UTF-16"),
            String::from_utf16(&value).expect("a value is UTF-16"),
        ));
        return;
    }
    cursor = (cursor + value_len).next_multiple_of(4);
    while cursor < end {
        strings(block, cursor, out);
        cursor = (cursor + u16_at(block, cursor)).next_multiple_of(4);
    }
}

fn version_string(key: &str) -> String {
    version_string_in(&resources(), key)
}

fn version_string_in(resources: &[(u16, Vec<u8>)], key: &str) -> String {
    let blocks = kind_in(resources, RT_VERSION);
    let [block] = blocks.as_slice() else {
        panic!(
            "the binary carries one version resource, found {}",
            blocks.len()
        );
    };
    let mut found = Vec::new();
    strings(block, 0, &mut found);
    found
        .into_iter()
        .find(|(name, _)| name == key)
        .unwrap_or_else(|| panic!("the version information has a `{key}`"))
        .1
}

/// Asserts that `resources` carry every image of `ico`, and one icon naming
/// them all.
fn assert_icon(resources: &[(u16, Vec<u8>)], ico: &[u8]) {
    let linked = kind_in(resources, RT_ICON);
    let count = u16_at(ico, 4);
    assert!(count > 0, "the committed icon has images");
    for index in 0..count {
        let entry = 6 + index * 16;
        let len = u32_at(ico, entry + 8);
        let offset = u32_at(ico, entry + 12);
        let image = &ico[offset..offset + len];
        assert!(
            linked.iter().any(|data| data == image),
            "image {index} of the committed icon is in the binary"
        );
    }
    assert_eq!(linked.len(), count, "and no image of another icon");

    let groups = kind_in(resources, RT_GROUP_ICON);
    let [group] = groups.as_slice() else {
        panic!("the binary carries one icon, found {}", groups.len());
    };
    assert_eq!(
        u16_at(group, 4),
        count,
        "the icon names every image of the file"
    );
}

#[test]
fn the_icon_is_this_profiles_committed_file_image_for_image() {
    assert_icon(&resources(), ICON);
}

/// `nvs build --compile app.nvs` run in a new folder holding `files`, and the
/// executable it wrote there. The program prints `ready`.
fn bundle(name: &str, files: &[(&str, &[u8])]) -> (nvs_repo::Scratch, PathBuf) {
    let dir = nvs_repo::scratch(&format!("winres-{name}"));
    std::fs::write(dir.join("app.nvs"), "<?nvs\necho \"ready\\n\";\n").expect("writes");
    for (file, bytes) in files {
        std::fs::write(dir.join(file), bytes).expect("writes");
    }
    let exe = dir.join("app.exe");
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .args(["build", "--compile", "app.nvs", "-o"])
        .arg(&exe)
        .current_dir(&*dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(
        out.status.success(),
        "the program builds: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "and with no warning: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (dir, exe)
}

/// Runs `exe` in its own folder, and asserts it prints what the program does.
fn assert_runs(exe: &Path) {
    let out = nvs_repo::spawn(exe, &[])
        .current_dir(exe.parent().expect("a folder"))
        .output()
        .expect("the bundle is an executable this host can run");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "ready\n",
        "the bundle runs its program: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A bundle with a `package.toml` names itself after the package, at its
/// version, with the icon `[bundle]` names, and still loads and runs.
#[test]
fn a_bundle_carries_the_programs_name_version_and_icon() {
    let manifest =
        b"[package]\nname = 'Acme\\Shop'\nversion = \"2.3.4\"\n\n[bundle]\nicon = \"shop.ico\"\n";
    let (_dir, exe) = bundle(
        "manifest",
        &[("package.toml", manifest), ("shop.ico", OTHER_ICON)],
    );
    let found = resources_of(&exe);
    assert_eq!(version_string_in(&found, "ProductName"), "Acme\\Shop");
    assert_eq!(version_string_in(&found, "FileDescription"), "Acme\\Shop");
    assert_eq!(version_string_in(&found, "FileVersion"), "2.3.4");
    assert_eq!(version_string_in(&found, "ProductVersion"), "2.3.4");
    assert_eq!(version_string_in(&found, "OriginalFilename"), "app.exe");
    assert_icon(&found, OTHER_ICON);
    assert_eq!(
        kind_in(&found, RT_MESSAGETABLE),
        of_kind(RT_MESSAGETABLE),
        "the host's other resources are kept"
    );
    assert_runs(&exe);
}

/// Without a `package.toml`, a bundle carries the host's icon and version
/// information unchanged.
#[test]
fn a_bundle_without_a_manifest_carries_the_hosts_resource() {
    let (_dir, exe) = bundle("plain", &[]);
    let found = resources_of(&exe);
    assert_eq!(version_string_in(&found, "ProductName"), "Novis");
    assert_eq!(
        version_string_in(&found, "FileVersion"),
        env!("CARGO_PKG_VERSION")
    );
    assert_icon(&found, ICON);
    assert_runs(&exe);
}

#[test]
fn the_version_information_is_the_manifests_and_the_licenses() {
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(version_string("ProductName"), "Novis");
    assert_eq!(version_string("FileVersion"), version);
    assert!(version_string("ProductVersion").starts_with(version));
    assert_eq!(version_string("OriginalFilename"), "nvs.exe");
    assert_eq!(
        version_string("CompanyName"),
        env!("CARGO_PKG_AUTHORS").replace(':', ", ")
    );
    assert_eq!(
        version_string("FileDescription"),
        if cfg!(debug_assertions) {
            "Novis (debug build)"
        } else {
            "Novis"
        }
    );

    let holder = LICENSE
        .lines()
        .find_map(|line| line.trim().strip_prefix("Copyright (c) "))
        .expect("LICENSE has its copyright line");
    let copyright = version_string("LegalCopyright");
    assert!(
        copyright.contains(holder),
        "`{copyright}` names `{holder}`, LICENSE's own line"
    );
}

/// Every event-log record a hosted service writes (`src/dispatch.rs`) is
/// rendered as its one insertion string: the table covers ids 1 to 6 in one
/// block, and each entry's text is `%1`, as UTF-16.
#[test]
fn the_message_table_renders_every_service_record_as_its_insertion_string() {
    let tables = of_kind(RT_MESSAGETABLE);
    let [table] = tables.as_slice() else {
        panic!(
            "the binary carries one message table, found {}",
            tables.len()
        );
    };
    assert_eq!(u32_at(table, 0), 1, "one block");
    let (low, high, entries) = (u32_at(table, 4), u32_at(table, 8), u32_at(table, 12));
    assert_eq!((low, high), (1, 6), "the ids the service writes");
    let mut cursor = entries;
    for id in low..=high {
        let length = u16_at(table, cursor);
        assert_eq!(u16_at(table, cursor + 2), 1, "entry {id} is UTF-16");
        let text: Vec<u16> = table[cursor + 4..cursor + length]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|unit| *unit != 0)
            .collect();
        assert_eq!(
            String::from_utf16(&text).expect("UTF-16"),
            "%1",
            "entry {id}"
        );
        cursor += length;
    }
}
