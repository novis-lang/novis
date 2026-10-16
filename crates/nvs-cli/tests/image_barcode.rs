//! `QrCode::giroCode`, the Novis-source member of `Novis\Image` that builds the EPC payment
//! text a QR code carries, run through the built binary with no `nvs.toml`, as
//! `rule:core-classes/image-barcode-roster` places it. The codes themselves are drawn by the
//! component, and `crates/nvs-ext/tests/image_barcode.rs` reads them back.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory of the case's own under the target's scratch, emptied first.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("image-barcode")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `body` after the `use` line for `QrCode` and returns its stdout, failing the test on a
/// non-zero exit.
fn run(case: &str, body: &str) -> String {
    let dir = scratch(case);
    let program = format!("<?nvs\nuse Novis\\Image\\QrCode;\n{body}");
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

/// The payload is the eleven lines of the EPC layout, version 002 in UTF-8: the service tag,
/// the BIC, the name, the IBAN without spaces, the amount in euros with two decimals, an empty
/// purpose, then the reference or the text. A wrong IBAN checksum, a reference beside a text and
/// an amount with three decimals each throw a `LogicError`.
// covers: Novis\Image\QrCode::giroCode
#[test]
fn an_epc_payload_is_the_girocode_layout() {
    let out = run(
        "girocode",
        r##"
echo QrCode::giroCode({name: "Shop GmbH", iban: "DE89 3704 0044 0532 0130 00", amount: 12.5, bic: "cobadeffxxx", text: "Order 42"}), "|\n";
echo QrCode::giroCode({name: "Blog", iban: "DE89370400440532013000", reference: "RF18539007547034"}), "|\n";
foreach ([
    fn() => QrCode::giroCode({name: "Shop", iban: "DE89370400440532013001"}),
    fn() => QrCode::giroCode({name: "Shop", iban: "DE89370400440532013000", reference: "RF18", text: "Order"}),
    fn() => QrCode::giroCode({name: "Shop", iban: "DE89370400440532013000", amount: 1.005}),
] as var $call) {
    try {
        $call();
        echo "no error\n";
    } catch (LogicError $e) {
        echo "LogicError\n";
    }
}
"##,
    );
    assert_eq!(
        out,
        concat!(
            "BCD\n002\n1\nSCT\nCOBADEFFXXX\nShop GmbH\nDE89370400440532013000\nEUR12.50\n\n\nOrder 42|\n",
            "BCD\n002\n1\nSCT\n\nBlog\nDE89370400440532013000\n\n\nRF18539007547034\n|\n",
            "LogicError\nLogicError\nLogicError\n",
        )
    );
}
