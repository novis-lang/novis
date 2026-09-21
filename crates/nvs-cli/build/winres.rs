//! The compiled Windows resource file `build.rs` links into `nvs.exe`: the
//! icon Explorer and the taskbar draw, and the version information behind a
//! file's *Properties → Details*, Task Manager's process name and the firewall
//! prompt `nvs serve` raises the first time it listens.
//!
//! This writes the `.res` format the resource compiler writes, by hand, and
//! the MSVC linker takes that file as an ordinary input. The alternative is a
//! build dependency that finds and runs `rc.exe`; the format is three fixed
//! layouts, and this is a build script for a compiler whose supply chain is
//! the thing it advertises. `rule:packaging/the-windows-binary-says-what-it-is`
//! owns what goes in, and `tests/windows_resource.rs` reads it back out of the
//! linked binary.
//!
//! Every length in these layouts is a fixed-width field, so every function
//! here returns `None` for an input too large for its field instead of
//! truncating it, and `build.rs` links nothing in that case.

/// `RT_ICON`: one image of an icon.
const RT_ICON: u16 = 3;
/// `RT_GROUP_ICON`: the directory naming the `RT_ICON` images of one icon.
const RT_GROUP_ICON: u16 = 14;
/// `RT_VERSION`: the version information.
const RT_VERSION: u16 = 16;

/// US English, the language the resource compiler stamps by default, and with
/// [`CODEPAGE`] the one translation the version information declares.
const LANGUAGE: u16 = 0x0409;
/// Unicode, which is what every string below is written as.
const CODEPAGE: u16 = 0x04B0;

/// `VS_FF_DEBUG` and `VS_FF_PRERELEASE`, and the mask declaring which flag
/// bits are meaningful.
const FLAG_DEBUG: u32 = 0x1;
const FLAG_PRERELEASE: u32 = 0x2;
const FLAGS_MASK: u32 = 0x3F;

/// What the version information says.
pub(crate) struct VersionInfo<'a> {
    /// `major.minor.patch`, the numeric version both fixed fields carry.
    pub(crate) version: [u16; 3],
    /// Whether this is a debug build, which sets `VS_FF_DEBUG`.
    pub(crate) debug: bool,
    /// Whether the version carries a pre-release tag, which sets
    /// `VS_FF_PRERELEASE`.
    pub(crate) prerelease: bool,
    /// The named strings, in the order they are written. A string whose value
    /// is empty is left out.
    pub(crate) strings: &'a [(&'a str, &'a str)],
}

/// The whole `.res` file: every image of `ico` as icon `1`, and `info` as
/// version resource `1`.
///
/// `None` if `ico` is not an icon file or a length does not fit its field.
pub(crate) fn resource(ico: &[u8], info: &VersionInfo<'_>) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    // A `.res` file opens on one empty entry, which is how a reader tells the
    // 32-bit format from the 16-bit one.
    entry(&mut out, 0, 0, 0, 0, &[])?;

    let mut group = Vec::new();
    let images = images(ico)?;
    group.extend_from_slice(&[0, 0, 1, 0]);
    push_u16(&mut group, u16::try_from(images.len()).ok()?);
    for (index, image) in images.iter().enumerate() {
        // Image ids start at 1; the group entry is the file's own directory
        // entry with the image's offset replaced by that id.
        let id = u16::try_from(index + 1).ok()?;
        entry(&mut out, RT_ICON, id, 0x1010, LANGUAGE, image.data)?;
        group.extend_from_slice(image.directory);
        push_u16(&mut group, id);
    }
    entry(&mut out, RT_GROUP_ICON, 1, 0x1030, LANGUAGE, &group)?;
    entry(&mut out, RT_VERSION, 1, 0x0030, LANGUAGE, &version(info)?)?;
    Some(out)
}

/// One image of an `.ico` file.
struct Image<'a> {
    /// The first twelve bytes of its directory entry: width, height, colours,
    /// a reserved byte, planes, bit count and the byte length of `data`.
    directory: &'a [u8],
    data: &'a [u8],
}

/// The images of an `.ico` file, or `None` if it is not one.
fn images(ico: &[u8]) -> Option<Vec<Image<'_>>> {
    // Reserved zero, type 1 for an icon, then the image count.
    if ico.get(..4)? != [0, 0, 1, 0] {
        return None;
    }
    let count = usize::from(read_u16(ico, 4)?);
    let mut images = Vec::with_capacity(count);
    for index in 0..count {
        let at = 6 + index * 16;
        let directory = ico.get(at..at + 12)?;
        let len = usize::try_from(read_u32(ico, at + 8)?).ok()?;
        let offset = usize::try_from(read_u32(ico, at + 12)?).ok()?;
        let data = ico.get(offset..offset.checked_add(len)?)?;
        images.push(Image { directory, data });
    }
    (!images.is_empty()).then_some(images)
}

/// The `VS_VERSIONINFO` block: the fixed fields, the string table and the
/// translation that names the table.
fn version(info: &VersionInfo<'_>) -> Option<Vec<u8>> {
    let [major, minor, patch] = info.version;
    let most = u32::from(major) << 16 | u32::from(minor);
    let least = u32::from(patch) << 16;
    let mut flags = 0;
    if info.debug {
        flags |= FLAG_DEBUG;
    }
    if info.prerelease {
        flags |= FLAG_PRERELEASE;
    }

    let mut fixed = Vec::new();
    for field in [
        0xFEEF_04BD, // the structure's signature
        0x0001_0000, // and its version
        most,
        least, // the file version
        most,
        least, // the product version
        FLAGS_MASK,
        flags,
        0x0004_0004, // VOS_NT_WINDOWS32
        0x1,         // VFT_APP
        0,           // no subtype
        0,
        0, // no date: `rule:packaging/a-build-records-no-timestamp`
    ] {
        push_u32(&mut fixed, field);
    }

    let mut strings = Vec::new();
    for (key, value) in info.strings {
        if !value.is_empty() {
            strings.push(node(key, Value::Text(value), &[])?);
        }
    }
    // The table's key is its language and code page as eight hex digits.
    let table = node(
        &format!("{LANGUAGE:04X}{CODEPAGE:04X}"),
        Value::None,
        &strings,
    )?;
    let string_info = node("StringFileInfo", Value::None, &[table])?;

    let mut translation = Vec::new();
    push_u16(&mut translation, LANGUAGE);
    push_u16(&mut translation, CODEPAGE);
    let var = node("Translation", Value::Binary(&translation), &[])?;
    let var_info = node("VarFileInfo", Value::None, &[var])?;

    node(
        "VS_VERSION_INFO",
        Value::Binary(&fixed),
        &[string_info, var_info],
    )
}

/// What one node of the version block carries beside its key.
enum Value<'a> {
    None,
    Binary(&'a [u8]),
    Text(&'a str),
}

/// One node of the version block: its length, its value's length, whether the
/// value is text, its key, its value and its children, each of the last three
/// starting on a 32-bit boundary.
///
/// Every node starts on such a boundary itself, so padding to a multiple of
/// four from the node's own start is padding within the whole block. The
/// length covers the children and the padding between them, and not the
/// padding after the last one.
fn node(key: &str, value: Value<'_>, children: &[Vec<u8>]) -> Option<Vec<u8>> {
    let (value, value_len, text) = match value {
        Value::None => (Vec::new(), 0, true),
        Value::Binary(bytes) => (bytes.to_vec(), bytes.len(), false),
        Value::Text(text) => {
            let wide = wide(text);
            // A text value's length counts characters, the terminator included.
            let len = wide.len() / 2;
            (wide, len, true)
        }
    };

    let mut out = vec![0; 6];
    out.extend_from_slice(&wide(key));
    pad(&mut out);
    out.extend_from_slice(&value);
    for child in children {
        pad(&mut out);
        out.extend_from_slice(child);
    }

    let len = u16::try_from(out.len()).ok()?;
    let value_len = u16::try_from(value_len).ok()?;
    out[0..2].copy_from_slice(&len.to_le_bytes());
    out[2..4].copy_from_slice(&value_len.to_le_bytes());
    out[4..6].copy_from_slice(&u16::from(text).to_le_bytes());
    Some(out)
}

/// One entry of the `.res` file: a 32-byte header naming the resource by
/// numeric type and id, then its data, padded to a 32-bit boundary.
fn entry(
    out: &mut Vec<u8>,
    kind: u16,
    id: u16,
    memory_flags: u16,
    language: u16,
    data: &[u8],
) -> Option<()> {
    push_u32(out, u32::try_from(data.len()).ok()?);
    push_u32(out, 32);
    // `0xFFFF` says the type, and then the name, is a number and not a string.
    push_u16(out, 0xFFFF);
    push_u16(out, kind);
    push_u16(out, 0xFFFF);
    push_u16(out, id);
    push_u32(out, 0); // data version
    push_u16(out, memory_flags);
    push_u16(out, language);
    push_u32(out, 0); // version
    push_u32(out, 0); // characteristics
    out.extend_from_slice(data);
    pad(out);
    Some(())
}

/// `text` as NUL-terminated UTF-16, little endian.
fn wide(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn pad(out: &mut Vec<u8>) {
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}
