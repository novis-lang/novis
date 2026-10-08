//! `Novis\Image\Color`, the Novis-source half of the built-in image component,
//! run through the built binary with no `nvs.toml`: the channels each
//! constructor reads, and the `LogicError` each one throws for a value that
//! is not a colour, as `rule:core-classes/image-pipeline` places them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("image-color")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `body` after a `use Novis\Image\Color;` line and returns its stdout,
/// failing the test on a non-zero exit.
fn run(case: &str, body: &str) -> String {
    let dir = scratch(case);
    let program = format!("<?nvs\nuse Novis\\Image\\Color;\n{body}");
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

/// `rgba` keeps the channels it is given and defaults alpha to 1, and `hex`
/// reads all four lengths with or without the `#`, doubling each digit of
/// the short ones.
// covers: Novis\Image\Color::rgba, Novis\Image\Color::hex
#[test]
fn rgba_and_hex_read_the_same_colour() {
    let out = run(
        "same-colour",
        r##"
array<Color> $colors = [
    Color::rgba(255, 136, 0),
    Color::hex("#ff8800"),
    Color::hex("f80"),
    Color::hex("#ff8800ff"),
    Color::hex("F80F"),
    Color::rgba(255, 136, 0, 0.5),
];
foreach ($colors as Color $c) {
    echo $c->r, " ", $c->g, " ", $c->b, " ", $c->alpha, "\n";
}
"##,
    );
    assert_eq!(
        out,
        "255 136 0 1\n255 136 0 1\n255 136 0 1\n255 136 0 1\n255 136 0 1\n255 136 0 0.5\n"
    );
}

/// A channel past 255, an alpha that is `NAN` or outside 0.0 to 1.0, and
/// text that is not 3, 4, 6 or 8 hex digits each throw a `LogicError`, and
/// none of them reaches the guest.
// covers: Novis\Image\Color::rgba, Novis\Image\Color::hex
#[test]
fn a_value_that_is_not_a_colour_throws_a_logic_error() {
    let out = run(
        "not-a-colour",
        r##"
class Check {
    public static function rgba(uint $r, float $alpha): string {
        try {
            Color::rgba($r, 0, 0, $alpha);
            return "colour";
        } catch (LogicError $e) {
            return "LogicError";
        }
    }

    public static function hex(string $text): string {
        try {
            Color::hex($text);
            return "colour";
        } catch (LogicError $e) {
            return "LogicError";
        }
    }
}

echo Check::rgba(256, 1.0), " ", Check::rgba(0, -0.01), " ", Check::rgba(0, 1.01), " ", Check::rgba(0, Core\Math::NAN), "\n";
echo Check::hex(""), " ", Check::hex("#12"), " ", Check::hex("#12345"), " ", Check::hex("#xyzxyz"), " ", Check::hex("#123456789"), "\n";
"##,
    );
    assert_eq!(
        out,
        "LogicError LogicError LogicError LogicError\n\
         LogicError LogicError LogicError LogicError LogicError\n"
    );
}
