//! `Novis\Pdf\Reader` and `Novis\Pdf\Pages`, run through the built binary: each takes a
//! `Core\Bytes::Source`, so a PDF opens from an open file and from an uploaded part, the host reads
//! the source whole at the call, and the text read from the file is `tainted`
//! (`rule:core-api/a-member-takes-the-source-the-program-holds`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("pdf")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `nvs <args> main.nvs` in `dir` over `body` after the `use` lines.
fn nvs(dir: &Path, args: &[&str], body: &str) -> Output {
    let program = format!(
        "<?nvs\nuse Novis\\Pdf\\Document;\nuse Novis\\Pdf\\Pages;\nuse Novis\\Pdf\\Reader;\n{body}"
    );
    fs::write(dir.join("main.nvs"), program).unwrap();
    let data = nvs_repo::scratch_private("nvsdata");
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .args(args)
        .arg("main.nvs")
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// [`nvs`] under `run`, failing the test on a non-zero exit, and its stdout.
fn run(dir: &Path, args: &[&str], body: &str) -> String {
    let mut all = vec!["run"];
    all.extend_from_slice(args);
    let out = nvs(dir, &all, body);
    assert!(
        out.status.success(),
        "the program runs: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("the output is UTF-8")
}

/// Renders a one-page PDF whose text is `Hello` into `hello.pdf` in `dir`, and returns its bytes.
fn hello_pdf(dir: &Path) -> Vec<u8> {
    run(
        dir,
        &[],
        "Core\\IO::write('hello.pdf', Document::fromHtml('<p>Hello</p>')->render()->bytes);",
    );
    fs::read(dir.join("hello.pdf")).expect("the program wrote the PDF")
}

/// A `POST` request whose body is one multipart part named `file`, holding `content` byte for
/// byte, in `nvs_test::request`'s format.
fn upload(content: &[u8]) -> Vec<u8> {
    let mut request = b"--METHOD--\nPOST\n--PATH--\n/upload\n--HEADERS--\nhost: example.com\n\
content-type: multipart/form-data; boundary=pdf-5c1e\n--BODY--\n"
        .to_vec();
    request.extend_from_slice(
        b"--pdf-5c1e\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.pdf\"\r\n\
Content-Type: application/pdf\r\n\r\n",
    );
    request.extend_from_slice(content);
    request.extend_from_slice(b"\r\n--pdf-5c1e--\r\n");
    request
}

/// `Reader::open` and `Pages::of` read an open file whole, from its start, and put its position
/// back, so the file reads as its bytes do.
// covers: Novis\Pdf\Reader::open, Novis\Pdf\Pages::of
#[test]
fn a_pdf_opens_from_a_file() {
    let dir = scratch("file");
    hello_pdf(&dir);
    let out = run(
        &dir,
        &[],
        r#"
Core\IO\File $file = Core\IO::open('hello.pdf', Core\IO::FileMode::Read);
$file->readBytes(4);
var $reader = Reader::open($file);
echo $reader->pageCount, ' ', $reader->text(1), ' ', $file->tell(), "\n";
bytes $merged = Pages::merge([Pages::of($file), Pages::of(Core\IO::readBytes('hello.pdf'))]);
echo Reader::open($merged)->pageCount, "\n";
"#,
    );
    assert_eq!(out, "1 Hello 4\n2\n");
}

/// A PDF uploaded as a part opens from the part, and its text is `tainted`: a program that puts
/// it in a plain `string` does not compile.
// covers: Novis\Pdf\Reader::open, Novis\Pdf\Reader::text
#[test]
fn a_pdf_from_a_part_gives_tainted_text() {
    let dir = scratch("part");
    let pdf = hello_pdf(&dir);
    fs::write(dir.join("upload.nvsr"), upload(&pdf)).unwrap();
    let out = run(
        &dir,
        &["--request", "upload.nvsr"],
        r#"
foreach (Core\Request::files() as Core\Request\Part $part) {
    tainted string $text = Reader::open($part)->text(1);
    echo $text, "\n";
}
"#,
    );
    assert_eq!(out, "Hello\n");
    let refused = nvs(
        &dir,
        &["check"],
        r#"
foreach (Core\Request::files() as Core\Request\Part $part) {
    string $text = Reader::open($part)->text(1);
    echo $text, "\n";
}
"#,
    );
    assert!(!refused.status.success(), "plain text does not compile");
    let message = String::from_utf8_lossy(&refused.stderr);
    assert!(message.contains("tainted string"), "{message}");
}
