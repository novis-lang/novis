//! `rule:packaging/the-first-party-components-are-built-in` in `nvs serve`: a served program's
//! built-in components are loaded before the server accepts. The server prints one line per
//! component it loaded, and every such line comes before its first `listening on` line, which it
//! prints once its sockets are bound.

use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// How long `nvs serve` has to compile its program, load the component, and bind its port. A
/// cold cache compiles the spreadsheet component in a debug build.
const BOOT: Duration = Duration::from_secs(180);

/// A program that calls the spreadsheet component and no other.
const PROGRAM: &str = "<?nvs\nuse Novis\\Spreadsheet\\Workbook;\n\necho Workbook::fromRows([['a' => 1]], { format: 'csv' })->bytes() as string;\n";

#[test]
fn a_served_program_loads_the_built_in_components_it_calls_before_it_accepts() {
    let dir = nvs_repo::scratch_private("builtin-warm");
    std::fs::write(dir.join("app.nvs"), PROGRAM).expect("the program is written");
    let data = nvs_repo::scratch_private("builtin-warm-data");
    let mut child = nvs_repo::spawn(env!("CARGO_BIN_EXE_nvs"), &[])
        .arg("--data")
        .arg(&*data)
        .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
        .current_dir(&*dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("the `nvs` binary this test was built beside starts");
    let out = child.stdout.take().expect("standard output is piped");
    let (sent, lines) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if sent.send(line).is_err() {
                break;
            }
        }
    });
    let mut before = Vec::new();
    let listening = loop {
        match lines.recv_timeout(BOOT) {
            Ok(line) if line.starts_with("listening on ") => break Some(line),
            Ok(line) => before.push(line),
            Err(_) => break None,
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        listening.is_some(),
        "the server never listened; it printed {before:?}"
    );
    assert_eq!(
        before
            .iter()
            .filter(|line| line.starts_with("loaded the built-in component "))
            .collect::<Vec<_>>(),
        ["loaded the built-in component `spreadsheet`"],
        "the server printed {before:?} before it listened"
    );
}
