//! A Windows bundle's own icon and version information
//! (`rule:packaging/the-windows-binary-says-what-it-is`).
//!
//! The copy of `nvs.exe` carries the host's resources. [`rewrite`] reads them
//! with `object`, replaces the version information and, when the program names
//! one, the icon, and writes the whole resource tree into a new section at the
//! end of the image. The resource data directory then names the new section,
//! and the old one stays in the file unread. The layouts of the icon and the
//! version block are `build/winres.rs`'s, which this module includes, so the
//! host and a bundle are written by one piece of code.
//!
//! The section is appended rather than the old one rewritten in place: every
//! section after `.rsrc`, such as `.reloc`, would otherwise move, and each of
//! their addresses with it. A certificate table is dropped, since any change to
//! the image breaks the signature it carries.
//!
//! Cost: the host's resources once more in every bundle, a few tens of
//! kilobytes, and the same again in memory while `nvs build --compile` runs.
//! Nothing here runs when a bundle starts.

use std::collections::BTreeMap;

use object::LittleEndian as LE;
use object::read::pe::{PeFile64, ResourceDirectoryEntryData, ResourceNameOrId};

#[path = "../../build/winres.rs"]
#[allow(
    dead_code,
    reason = "build.rs writes the whole `.res` file, and a bundle needs the icon and version layouts alone"
)]
mod winres;

/// What a bundle says about itself.
pub(super) struct Program<'a> {
    /// `ProductName` and `FileDescription`.
    pub(super) name: &'a str,
    /// `MAJOR.MINOR.PATCH`, which `FileVersion` and `ProductVersion` carry as
    /// text. A part over 65535 is written as 65535 in the numeric fields.
    pub(super) version: [u64; 3],
    /// The executable's file name, `OriginalFilename`.
    pub(super) file: &'a str,
    /// An `.ico` file's bytes, or `None` to keep the host's icon.
    pub(super) icon: Option<&'a [u8]>,
}

/// Whether `bytes` is an `.ico` file this module can write as an icon.
pub(super) fn is_icon(bytes: &[u8]) -> bool {
    winres::icon(bytes).is_some()
}

/// Every resource of an image: type, then id, then language, then the data.
type Tree = BTreeMap<u16, BTreeMap<u16, BTreeMap<u16, Vec<u8>>>>;

/// `host` with `program`'s resources.
///
/// `Ok(None)` when `host` is not a 64-bit PE image, which is every host but a
/// Windows one.
///
/// # Errors
///
/// Why the image cannot take a new section, or a resource this module does
/// not read, such as one named by a string.
pub(super) fn rewrite(host: &[u8], program: &Program<'_>) -> Result<Option<Vec<u8>>, String> {
    let Ok(file) = PeFile64::parse(host) else {
        return Ok(None);
    };
    let mut tree = resources(&file, host)?;

    let text = format!(
        "{}.{}.{}",
        program.version[0], program.version[1], program.version[2]
    );
    let internal = program
        .file
        .rsplit_once('.')
        .map_or(program.file, |(stem, _)| stem);
    let strings = [
        ("ProductName", program.name),
        ("FileDescription", program.name),
        ("FileVersion", text.as_str()),
        ("ProductVersion", text.as_str()),
        ("InternalName", internal),
        ("OriginalFilename", program.file),
    ];
    let info = winres::VersionInfo {
        version: program
            .version
            .map(|part| u16::try_from(part).unwrap_or(u16::MAX)),
        debug: false,
        prerelease: false,
        strings: &strings,
    };
    let version = winres::version_resource(&info).ok_or("the version information is too long")?;
    let mut new = vec![version];
    if let Some(ico) = program.icon {
        tree.remove(&winres::RT_ICON);
        tree.remove(&winres::RT_GROUP_ICON);
        new.extend(winres::icon(ico).ok_or("the icon is not an `.ico` file")?);
    }
    tree.remove(&winres::RT_VERSION);
    for resource in new {
        tree.entry(resource.kind)
            .or_default()
            .entry(resource.id)
            .or_default()
            .insert(winres::LANGUAGE, resource.data);
    }
    append(host, &tree).map(Some)
}

/// Every resource `file` carries.
fn resources(file: &PeFile64<'_>, host: &[u8]) -> Result<Tree, String> {
    let malformed = |err: object::Error| format!("its resources do not parse: {err}");
    let sections = file.section_table();
    let mut tree = Tree::new();
    let Some(directory) = file
        .data_directories()
        .resource_directory(host, &sections)
        .map_err(malformed)?
    else {
        return Ok(tree);
    };
    let id = |name: ResourceNameOrId| match name {
        ResourceNameOrId::Id(id) => Ok(id),
        ResourceNameOrId::Name(_) => Err("a resource is named by a string".to_owned()),
    };
    let table = |data| match data {
        ResourceDirectoryEntryData::Table(table) => Ok(table),
        ResourceDirectoryEntryData::Data(_) => Err("a resource directory ends early".to_owned()),
    };
    for kind in directory.root().map_err(malformed)?.entries {
        let ids = table(kind.data(directory).map_err(malformed)?)?;
        for name in ids.entries {
            let languages = table(name.data(directory).map_err(malformed)?)?;
            for language in languages.entries {
                let ResourceDirectoryEntryData::Data(leaf) =
                    language.data(directory).map_err(malformed)?
                else {
                    return Err("a resource directory is nested too deep".to_owned());
                };
                let bytes = usize::try_from(leaf.size.get(LE))
                    .ok()
                    .and_then(|len| {
                        sections
                            .pe_data_at(host, leaf.offset_to_data.get(LE))?
                            .get(..len)
                    })
                    .ok_or("a resource's data is outside its image")?;
                tree.entry(id(kind.name_or_id())?)
                    .or_default()
                    .entry(id(name.name_or_id())?)
                    .or_default()
                    .insert(id(language.name_or_id())?, bytes.to_vec());
            }
        }
    }
    Ok(tree)
}

/// Where the fields this module writes are, from the start of the optional
/// header of a PE32+ image.
mod at {
    pub(super) const SIZE_OF_INITIALIZED_DATA: usize = 8;
    pub(super) const SECTION_ALIGNMENT: usize = 32;
    pub(super) const FILE_ALIGNMENT: usize = 36;
    pub(super) const SIZE_OF_IMAGE: usize = 56;
    pub(super) const SIZE_OF_HEADERS: usize = 60;
    pub(super) const CHECKSUM: usize = 64;
    pub(super) const NUMBER_OF_RVA_AND_SIZES: usize = 108;
    pub(super) const DATA_DIRECTORIES: usize = 112;
}

/// The data directories this module writes.
const RESOURCE_DIRECTORY: usize = 2;
const SECURITY_DIRECTORY: usize = 4;
/// One section header's size.
const SECTION_HEADER: usize = 40;
/// `IMAGE_SCN_CNT_INITIALIZED_DATA | IMAGE_SCN_MEM_READ`, as the linker marks `.rsrc`.
const RESOURCE_SECTION: u32 = 0x4000_0040;

/// `host` cut at the end of its last section, with `tree` in a new section
/// after it, which the resource data directory names.
fn append(host: &[u8], tree: &Tree) -> Result<Vec<u8>, String> {
    let short = || "its headers are cut short".to_owned();
    let nt = read32(host, 0x3C).ok_or_else(short)?;
    let count = usize::from(read16(host, nt + 6).ok_or_else(short)?);
    let optional = nt + 24;
    let table = optional + usize::from(read16(host, nt + 20).ok_or_else(short)?);
    let field = |offset| read32(host, optional + offset).ok_or_else(short);
    let section_alignment = field(at::SECTION_ALIGNMENT)?;
    let file_alignment = field(at::FILE_ALIGNMENT)?;
    let size_of_headers = field(at::SIZE_OF_HEADERS)?;
    if field(at::NUMBER_OF_RVA_AND_SIZES)? <= SECURITY_DIRECTORY
        || section_alignment == 0
        || file_alignment == 0
    {
        return Err("its optional header is not one this module writes".to_owned());
    }

    let (mut raw_end, mut virtual_end, mut first_raw) = (0, 0, usize::MAX);
    let mut old = None;
    let resource_rva = field(at::DATA_DIRECTORIES + RESOURCE_DIRECTORY * 8)?;
    for index in 0..count {
        let header = table + index * SECTION_HEADER;
        let read = |offset| read32(host, header + offset).ok_or_else(short);
        let (size, address, raw_size, raw_at) = (read(8)?, read(12)?, read(16)?, read(20)?);
        if raw_size > 0 {
            first_raw = first_raw.min(raw_at);
            raw_end = raw_end.max(raw_at + raw_size);
        }
        virtual_end = virtual_end.max(address + size.max(raw_size));
        if (address..address + size.max(raw_size)).contains(&resource_rva) {
            old = Some(header);
        }
    }
    let header = table + count * SECTION_HEADER;
    let free = host
        .get(header..header + SECTION_HEADER)
        .ok_or_else(short)?;
    if header + SECTION_HEADER > size_of_headers.min(first_raw) || free.iter().any(|b| *b != 0) {
        return Err("its headers have no room for another section".to_owned());
    }
    if raw_end > host.len() {
        return Err(short());
    }

    let address = virtual_end.next_multiple_of(section_alignment);
    let raw_at = raw_end.next_multiple_of(file_alignment);
    let body = directory(tree, address).ok_or("its resources do not fit one section")?;
    let raw_size = body.len().next_multiple_of(file_alignment);

    // Everything after the last section, the certificate table included, is
    // cut off: § *What is appended* puts the payload there.
    let mut out = host[..raw_end].to_vec();
    out.resize(raw_at, 0);
    out.extend_from_slice(&body);
    out.resize(raw_at + raw_size, 0);

    let too_large = || "the new section is past the image's 4 GiB".to_owned();
    let wide = |value: usize| u32::try_from(value).map_err(|_| too_large());
    let mut section = [0_u8; SECTION_HEADER];
    section[..5].copy_from_slice(b".rsrc");
    section[8..12].copy_from_slice(&wide(body.len())?.to_le_bytes());
    section[12..16].copy_from_slice(&wide(address)?.to_le_bytes());
    section[16..20].copy_from_slice(&wide(raw_size)?.to_le_bytes());
    section[20..24].copy_from_slice(&wide(raw_at)?.to_le_bytes());
    section[36..40].copy_from_slice(&RESOURCE_SECTION.to_le_bytes());
    out[header..header + SECTION_HEADER].copy_from_slice(&section);
    // The old section keeps its bytes, under a name no tool looks up as the
    // resource section.
    if let Some(old) = old
        && out[old..old + 8] == *b".rsrc\0\0\0"
    {
        out[old..old + 8].copy_from_slice(b".oldrsrc");
    }

    let count = u16::try_from(count + 1).map_err(|_| "it has too many sections".to_owned())?;
    out[nt + 6..nt + 8].copy_from_slice(&count.to_le_bytes());
    let initialized = field(at::SIZE_OF_INITIALIZED_DATA)?.saturating_add(raw_size);
    let mut write = |offset: usize, value: u32| {
        out[optional + offset..optional + offset + 4].copy_from_slice(&value.to_le_bytes());
    };
    write(
        at::SIZE_OF_INITIALIZED_DATA,
        wide(initialized).unwrap_or(u32::MAX),
    );
    write(
        at::SIZE_OF_IMAGE,
        wide((address + body.len()).next_multiple_of(section_alignment))?,
    );
    // Windows checks the sum of a driver alone, and zero says there is none.
    write(at::CHECKSUM, 0);
    let resource = at::DATA_DIRECTORIES + RESOURCE_DIRECTORY * 8;
    write(resource, wide(address)?);
    write(resource + 4, wide(body.len())?);
    let security = at::DATA_DIRECTORIES + SECURITY_DIRECTORY * 8;
    write(security, 0);
    write(security + 4, 0);
    Ok(out)
}

/// `IMAGE_RESOURCE_DIRECTORY_ENTRY`'s flag saying its offset is another table.
const SUBDIRECTORY: u32 = 0x8000_0000;

/// `tree` as a resource section loaded at `address`: the tables of the three
/// levels, then one `IMAGE_RESOURCE_DATA_ENTRY` per resource, then the data,
/// each on an 8-byte boundary. Every name is a number, and every level is in
/// ascending order, as the loader's binary search needs.
///
/// `None` if an offset does not fit its 32-bit field.
fn directory(tree: &Tree, address: usize) -> Option<Vec<u8>> {
    let table_len = |entries: usize| 16 + 8 * entries;
    let mut next = table_len(tree.len());
    let mut type_tables = Vec::new();
    for ids in tree.values() {
        type_tables.push(next);
        next += table_len(ids.len());
    }
    let mut id_tables = Vec::new();
    for languages in tree.values().flat_map(BTreeMap::values) {
        id_tables.push(next);
        next += table_len(languages.len());
    }
    let leaves: Vec<(u16, &Vec<u8>)> = tree
        .values()
        .flat_map(BTreeMap::values)
        .flat_map(|languages| languages.iter().map(|(language, data)| (*language, data)))
        .collect();
    let leaf_start = next;
    let data_start = (leaf_start + 16 * leaves.len()).next_multiple_of(8);

    let mut out = Vec::new();
    let table = |out: &mut Vec<u8>, entries: &mut dyn Iterator<Item = (u16, usize, bool)>| {
        let entries: Vec<_> = entries.collect();
        out.extend_from_slice(&[0; 12]);
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&u16::try_from(entries.len()).ok()?.to_le_bytes());
        for (name, offset, nested) in entries {
            let offset = u32::try_from(offset).ok()?;
            out.extend_from_slice(&u32::from(name).to_le_bytes());
            let flag = if nested { SUBDIRECTORY } else { 0 };
            out.extend_from_slice(&(offset | flag).to_le_bytes());
        }
        Some(())
    };
    table(
        &mut out,
        &mut tree
            .keys()
            .zip(&type_tables)
            .map(|(kind, at)| (*kind, *at, true)),
    )?;
    let mut id_at = id_tables.iter();
    for ids in tree.values() {
        table(
            &mut out,
            &mut ids
                .keys()
                .map(|id| (*id, *id_at.next().unwrap_or(&0), true)),
        )?;
    }
    let mut leaf_at = leaf_start;
    for languages in tree.values().flat_map(BTreeMap::values) {
        table(
            &mut out,
            &mut languages.keys().map(|language| {
                let at = leaf_at;
                leaf_at += 16;
                (*language, at, false)
            }),
        )?;
    }

    let mut data_at = data_start;
    for (_, data) in &leaves {
        let rva = u32::try_from(address.checked_add(data_at)?).ok()?;
        out.extend_from_slice(&rva.to_le_bytes());
        out.extend_from_slice(&u32::try_from(data.len()).ok()?.to_le_bytes());
        out.extend_from_slice(&[0; 8]);
        data_at = (data_at + data.len()).next_multiple_of(8);
    }
    out.resize(data_start, 0);
    for (_, data) in &leaves {
        out.extend_from_slice(data);
        out.resize(out.len().next_multiple_of(8), 0);
    }
    Some(out)
}

fn read16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn read32(bytes: &[u8], at: usize) -> Option<usize> {
    let value = u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?);
    usize::try_from(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree written by [`directory`] reads back through `object` as the same tree.
    #[test]
    fn a_written_directory_reads_back_as_the_same_resources() {
        let mut tree = Tree::new();
        for (kind, id, language, data) in [
            (3, 1, 0x409, vec![1, 2, 3]),
            (3, 2, 0x409, vec![4; 9]),
            (14, 1, 0x409, vec![5; 20]),
            (16, 1, 0x407, vec![6; 7]),
            (16, 1, 0x409, vec![7; 8]),
        ] {
            tree.entry(kind)
                .or_default()
                .entry(id)
                .or_default()
                .insert(language, data);
        }
        let section = directory(&tree, 0x1000).expect("the tree fits");
        let parsed = object::read::pe::ResourceDirectory::new(&section);
        let mut found = Tree::new();
        for kind in parsed.root().expect("the root parses").entries {
            let ResourceDirectoryEntryData::Table(ids) = kind.data(parsed).expect("a type") else {
                panic!("a type is a table");
            };
            for name in ids.entries {
                let ResourceDirectoryEntryData::Table(languages) =
                    name.data(parsed).expect("an id")
                else {
                    panic!("an id is a table");
                };
                for language in languages.entries {
                    let ResourceDirectoryEntryData::Data(leaf) =
                        language.data(parsed).expect("a language")
                    else {
                        panic!("a language is a leaf");
                    };
                    let at = usize::try_from(leaf.offset_to_data.get(LE)).expect("fits") - 0x1000;
                    let len = usize::try_from(leaf.size.get(LE)).expect("fits");
                    let number = |n: ResourceNameOrId| match n {
                        ResourceNameOrId::Id(id) => id,
                        ResourceNameOrId::Name(_) => panic!("every name is a number"),
                    };
                    found
                        .entry(number(kind.name_or_id()))
                        .or_default()
                        .entry(number(name.name_or_id()))
                        .or_default()
                        .insert(
                            number(language.name_or_id()),
                            section[at..at + len].to_vec(),
                        );
                }
            }
        }
        assert_eq!(found, tree);
    }
}
