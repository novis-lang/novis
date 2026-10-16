//! The Novis-source members of `Novis\Image` that call no export —
//! `Image::hashDistance`, `Image::mime`, `Image::extension`, `Font::fromBytes`
//! and `new Diff` — run through the built binary with no `nvs.toml`, as
//! `rule:core-classes/image-pipeline` places them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("image-analysis")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `body` after the `use` lines for `Image` and `Font` and returns its
/// stdout, failing the test on a non-zero exit.
fn run(case: &str, body: &str) -> String {
    let dir = scratch(case);
    let program = format!("<?nvs\nuse Novis\\Image\\Font;\nuse Novis\\Image\\Image;\n{body}");
    fs::write(dir.join("main.nvs"), program).unwrap();
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .args(["run", "main.nvs"])
        .current_dir(&dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(
        out.status.success(),
        "the program runs: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("the output is UTF-8")
}

/// `hashDistance` counts the bits that differ across every byte, is 0 for two
/// equal hashes, and throws a `LogicError` for two hashes of different lengths.
// covers: Novis\Image\Image::hashDistance
#[test]
fn hash_distance_counts_differing_bits_and_refuses_two_lengths() {
    let out = run(
        "hash-distance",
        r##"
bytes $a = Core\Encoding::fromHex("00ff0f");
echo Image::hashDistance($a, $a), "\n";
echo Image::hashDistance($a, Core\Encoding::fromHex("ff00f0")), "\n";
echo Image::hashDistance($a, Core\Encoding::fromHex("01fe0f")), "\n";
try {
    Image::hashDistance($a, Core\Encoding::fromHex("00ff"));
    echo "no error\n";
} catch (LogicError $e) {
    echo "LogicError\n";
}
"##,
    );
    assert_eq!(out, "0\n24\n2\nLogicError\n");
}

/// `fromBytes` accepts each of the four headers a font file starts with and
/// keeps the bytes as given, and throws a `ParseError` for bytes shorter than
/// a font header or starting with anything else.
// covers: Novis\Image\Font::fromBytes
#[test]
fn from_bytes_accepts_the_four_font_headers_and_throws_parse_error_otherwise() {
    let out = run(
        "font-from-bytes",
        r##"
class Check {
    public static function font(string $hex): string {
        try {
            return "font " . Core\Bytes::length(Font::fromBytes(Core\Encoding::fromHex($hex))->data);
        } catch (ParseError $e) {
            return "ParseError";
        }
    }
}

string $rest = "0000000000000000";
echo Check::font("00010000" . $rest), "\n";
echo Check::font("4f54544f" . $rest), "\n";
echo Check::font("74727565" . $rest), "\n";
echo Check::font("74746366" . $rest), "\n";
echo Check::font("89504e47" . $rest), "\n";
echo Check::font("00010000"), "\n";
"##,
    );
    assert_eq!(
        out,
        "font 12\nfont 12\nfont 12\nfont 12\nParseError\nParseError\n"
    );
}

/// `mime` and `extension` give every `Format` case its own MIME type and its
/// file extension without the dot.
// covers: Novis\Image\Image::mime, Novis\Image\Image::extension
#[test]
fn mime_and_extension_name_every_format() {
    let out = run(
        "mime-and-extension",
        r##"
use Novis\Image\Format;

array<Format> $formats = [Format::Jpeg, Format::Png, Format::Webp, Format::Gif, Format::Avif, Format::Jxl, Format::Svg, Format::Pdf];
foreach ($formats as Format $format) {
    echo Image::mime($format), " ", Image::extension($format), "\n";
}
"##,
    );
    assert_eq!(
        out,
        "image/jpeg jpg\nimage/png png\nimage/webp webp\nimage/gif gif\n\
         image/avif avif\nimage/jxl jxl\nimage/svg+xml svg\napplication/pdf pdf\n"
    );
}

/// `new Diff` keeps the five values it is given, so a program can build the
/// result `Image::compare` returns, for example in a test of its own.
// covers: Novis\Image\Diff::constructor
#[test]
fn a_diff_keeps_the_five_values_it_is_made_from() {
    let out = run(
        "diff-constructor",
        r##"
use Novis\Image\Diff;

Diff $diff = new Diff(false, 3, 40, 0.75, null);
echo $diff->identical ? "same" : "differ", " ", $diff->differingPixels, " ", $diff->maxDelta, " ", $diff->ssim, " ", $diff->diff == null ? "no diff" : "a diff", "\n";
"##,
    );
    assert_eq!(out, "differ 3 40 0.75 no diff\n");
}
