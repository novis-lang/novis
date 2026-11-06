//! Setting up Novis works in any order: `nvs run`, `nvs init` and `nvs service install`, run in
//! each of their six orders, leave a data folder, a cache and a project file that pass
//! `rule:config/ownership-is-the-trust-boundary`'s check for the account a service runs as.
//!
//! Through the built binary, for the reason `init.rs` writes down: `nvs-cli` has no library
//! target. The install is `--dry-run`, because an install that is not a dry run registers a real
//! service, and an elevated CI runner would let it. The dry run still runs the install's whole
//! judgement of the service's configuration for its account, which is the boot's check. What it
//! leaves out is the folder it would create, and that is the same `Folder::prepare` that `nvs run`
//! and `nvs init` call, so whichever command runs first, the folder is made the same way.
//!
//! Windows only. On Linux a service runs as root or as `--account`, and the install gives every
//! path it grants to that account itself (`service.rs`'s `owners`), so no order can leave a path
//! the service refuses. The `owners` cases there pin that half.
#![cfg(windows)]

use std::path::Path;
use std::process::{Command, Output};

/// The account the service in this test runs as: not an administrator, so the owner test asks
/// the question the order could break.
const ACCOUNT: &str = r"NT AUTHORITY\LocalService";

/// One setup step, as an operator types it.
#[derive(Debug, Clone, Copy)]
enum Step {
    Run,
    Init,
    Install,
}

/// `nvs` with `--data <dir>/data`, run in `<dir>/app`.
fn nvs(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(dir.join("data"))
        .args(args)
        .current_dir(dir.join("app"))
        .output()
        .expect("the binary under test runs")
}

/// `step` in `dir`. The service's argv names the same data folder with a full path, as the install
/// reference tells an operator to.
fn take(dir: &Path, step: Step) -> Output {
    let entry = dir.join("app").join("index.nvs");
    let data = dir.join("data");
    match step {
        Step::Run => nvs(dir, &["run", "index.nvs"]),
        Step::Init => nvs(dir, &["init"]),
        Step::Install => nvs(
            dir,
            &[
                "service",
                "install",
                "nvs-order",
                "--account",
                ACCOUNT,
                "--dry-run",
                "--",
                "serve",
                &entry.to_string_lossy(),
                "--data",
                &data.to_string_lossy(),
            ],
        ),
    }
}

/// Every order of the three setup steps succeeds, and what they leave passes the check the
/// service makes at boot: a last dry-run install judges the configuration for the service's
/// account, and the data folder, its `nvs.toml`, its cache and the project file each pass
/// `trust::check_as` for that account.
// covers: tools:install/set-up-a-server
#[test]
fn every_order_of_run_init_and_install_leaves_a_setup_the_service_accepts() {
    use Step::{Init, Install, Run};
    let account = nvs_config::trust::Account::named(ACCOUNT).expect("LocalService exists");
    let orders = [
        [Run, Init, Install],
        [Run, Install, Init],
        [Init, Run, Install],
        [Init, Install, Run],
        [Install, Run, Init],
        [Install, Init, Run],
    ];
    for (n, order) in orders.iter().enumerate() {
        let dir = nvs_repo::scratch_private(&format!("setup-order-{n}"));
        let app = dir.join("app");
        std::fs::create_dir(&app).expect("the project folder");
        std::fs::write(app.join("index.nvs"), "echo \"ready\";\n").expect("the entry file");

        for &step in order {
            let out = take(&dir, step);
            assert!(
                out.status.success(),
                "{order:?}: {step:?} failed\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }

        let last = take(&dir, Install);
        assert!(
            last.status.success(),
            "{order:?}: the service's account refuses the setup: {}",
            String::from_utf8_lossy(&last.stderr)
        );
        let data = dir.join("data");
        let mut judged = vec![data.clone(), data.join("cache"), app.join("nvs.toml")];
        // `nvs run` writes the shipped file here only when it finds no project file, and the real
        // install writes it before the service is registered. When it is here, it is judged too.
        if data.join("nvs.toml").is_file() {
            judged.push(data.join("nvs.toml"));
        }
        for path in judged {
            if let Err(why) = nvs_config::trust::check_as(&path, &account) {
                panic!(
                    "{order:?}: `{}` fails for {ACCOUNT}: {}",
                    path.display(),
                    why.message()
                );
            }
        }
    }
}
