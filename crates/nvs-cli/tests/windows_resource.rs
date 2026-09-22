//! `rule:packaging/the-windows-binary-says-what-it-is`, read back out of the
//! linked binary: `build/winres.rs` writes the resource by hand, so what holds
//! it is a reader that shares none of that code walking the `.exe` the linker
//! produced. The icon is compared image for image with the committed `.ico`,
//! and the version strings with the manifest and `LICENSE` they are taken from.

#![cfg(all(windows, target_env = "msvc"))]

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

const LICENSE: &str = include_str!("../../../LICENSE");

/// Every resource in the built `nvs.exe`, as its numeric type and its data.
fn resources() -> Vec<(u16, Vec<u8>)> {
    let exe = std::fs::read(env!("CARGO_BIN_EXE_nvs")).expect("the built binary is readable");
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
    resources()
        .into_iter()
        .filter(|(found, _)| *found == kind)
        .map(|(_, data)| data)
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
            .chunks_exact(2)
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
    let blocks = of_kind(RT_VERSION);
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

#[test]
fn the_icon_is_this_profiles_committed_file_image_for_image() {
    let linked = of_kind(RT_ICON);
    let count = u16_at(ICON, 4);
    assert!(count > 0, "the committed icon has images");
    for index in 0..count {
        let entry = 6 + index * 16;
        let len = u32_at(ICON, entry + 8);
        let offset = u32_at(ICON, entry + 12);
        let image = &ICON[offset..offset + len];
        assert!(
            linked.iter().any(|data| data == image),
            "image {index} of the committed icon is in the binary"
        );
    }

    let groups = of_kind(RT_GROUP_ICON);
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
            .chunks_exact(2)
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
