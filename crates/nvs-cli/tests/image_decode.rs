//! `Novis\Image\Image`, run through the built binary: `open` and `info` take a
//! `Core\Bytes::Source`, so an image opens and is described from an open file and from an
//! uploaded part. The host reads the source whole at the call, a file keeps its position, and a
//! part is held, so the program can still save it
//! (`rule:core-api/a-member-takes-the-source-the-program-holds`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("image-decode")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `nvs run <args> main.nvs` in `dir` over `body` after the `use` lines, failing the test on
/// a non-zero exit, and returns its stdout.
fn run(dir: &Path, args: &[&str], body: &str) -> String {
    let program = format!(
        "<?nvs\nuse Novis\\Image\\Color;\nuse Novis\\Image\\Format;\nuse Novis\\Image\\Image;\n{body}"
    );
    fs::write(dir.join("main.nvs"), program).unwrap();
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("run")
        .args(args)
        .arg("main.nvs")
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(
        out.status.success(),
        "the program runs: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("the output is UTF-8")
}

/// `text` as a TOML basic string.
fn toml_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Writes a 30 by 20 PNG file into `a.png` in `dir`, and returns its bytes.
fn png(dir: &Path) -> Vec<u8> {
    run(
        dir,
        &[],
        "Core\\IO::write('a.png', Image::create(30, 20, Color::hex('#336699'))->format(Format::Png)->encode());",
    );
    fs::read(dir.join("a.png")).expect("the program wrote the PNG")
}

/// `Image::open` reads an open file whole, from its start, and puts its position back.
// covers: Novis\Image\Image::open
#[test]
fn an_image_opens_from_a_file() {
    let dir = scratch("open");
    png(&dir);
    let out = run(
        &dir,
        &[],
        r#"
Core\IO\File $file = Core\IO::open('a.png', Core\IO::FileMode::Read);
$file->readBytes(3);
var $raw = Image::open($file)->raw();
echo $raw->width, 'x', $raw->height, ' ', $file->tell(), "\n";
echo Image::compare($file, Core\IO::readBytes('a.png'))->identical ? 'same' : 'differ', "\n";
"#,
    );
    assert_eq!(out, "30x20 3\nsame\n");
}

/// `Image::info` reads an open file and puts its position back.
// covers: Novis\Image\Image::info
#[test]
fn image_info_reads_a_file() {
    let dir = scratch("info");
    png(&dir);
    let out = run(
        &dir,
        &[],
        r#"
Core\IO\File $file = Core\IO::open('a.png', Core\IO::FileMode::Read);
var $info = Image::info($file);
echo $info->format == Format::Png ? 'png' : 'other', ' ', $info->width, 'x', $info->height, ' ', $file->tell(), "\n";
"#,
    );
    assert_eq!(out, "png 30x20 0\n");
}

/// `Image::info` of an uploaded part reads the part without consuming it, so `saveTo` still
/// writes every byte of it.
// covers: Novis\Image\Image::info
#[test]
fn image_info_of_a_part_then_save_to_writes_the_whole_part() {
    let dir = scratch("part");
    let png = png(&dir);
    let mut request = b"--METHOD--\nPOST\n--PATH--\n/upload\n--HEADERS--\nhost: example.com\n\
content-type: multipart/form-data; boundary=img-5c1e\n--BODY--\n"
        .to_vec();
    request.extend_from_slice(
        b"--img-5c1e\r\nContent-Disposition: form-data; name=\"photo\"; filename=\"a.png\"\r\n\
Content-Type: image/png\r\n\r\n",
    );
    request.extend_from_slice(&png);
    request.extend_from_slice(b"\r\n--img-5c1e--\r\n");
    fs::write(dir.join("upload.nvsr"), request).unwrap();
    let grant = toml_string(&format!("{}{}", dir.display(), std::path::MAIN_SEPARATOR));
    fs::write(
        dir.join("nvs.toml"),
        format!("[capabilities.fs]\nwrite = [{grant}]\n"),
    )
    .unwrap();
    let out = run(
        &dir,
        &["--request", "upload.nvsr"],
        r#"
foreach (Core\Request::files() as Core\Request\Part $part) {
    var $info = Image::info($part);
    echo $info->width, 'x', $info->height, "\n";
    $part->saveTo('saved.png');
}
"#,
    );
    assert_eq!(out, "30x20\n");
    assert_eq!(fs::read(dir.join("saved.png")).unwrap(), png);
}
