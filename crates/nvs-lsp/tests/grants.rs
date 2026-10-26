//! The grants a route's hover lists, read off the configuration beside the program
//! (`rule:ide/a-route-hover-shows-the-programs-grants`).
//!
//! The `.lspt` cases under `tests/lsp/hover/` freeze the hover with an `nvs.toml` beside the
//! document. These are the answers a case cannot pin: with no `nvs.toml` at all, because the
//! suite's binary has a data folder whose file would be read, and an `nvs.toml` rewritten
//! between two hovers of one unchanged document.

use nvs_lsp::grants::of_program;

#[test]
fn with_no_nvs_toml_the_hover_names_the_default_policy() {
    let dir = nvs_repo::scratch("grants-none");
    let entry = dir.join("main.nvs");
    std::fs::write(&entry, "<?nvs\n").expect("the entry file is written");

    assert_eq!(
        of_program(&entry, None),
        "No `nvs.toml` was found for this program. It runs under the default policy:\n\n\
         - `fs.read`: files in this file's directory, except `nvs.toml` and names that start \
         with `.` (default)\n\
         - `fs.write`: only the directories `Core\\IO::temporaryDir` makes (default)\n\
         - `script.spawn`: files in this file's directory, outside its temporary directories \
         (default)\n\
         - `net.connect`: every host (default)\n\
         \nNo other grant is given."
    );
}

#[test]
fn a_rewritten_nvs_toml_changes_the_next_answer() {
    let dir = nvs_repo::scratch("grants-rewritten");
    let entry = dir.join("main.nvs");
    std::fs::write(&entry, "<?nvs\n").expect("the entry file is written");
    let config = dir.join("nvs.toml");

    std::fs::write(&config, "[capabilities.net]\nconnect = [\"api.example.com\"]\n")
        .expect("the configuration is written");
    let before = of_program(&entry, None);
    assert!(
        before.contains("- `net.connect`: `api.example.com`\n"),
        "{before}"
    );

    std::fs::write(&config, "[capabilities.net]\nconnect = false\n")
        .expect("the configuration is rewritten");
    let after = of_program(&entry, None);
    assert!(after.contains("- `net.connect`: nothing\n"), "{after}");
}

#[test]
fn an_nvs_toml_that_does_not_load_is_named_and_lists_nothing() {
    let dir = nvs_repo::scratch("grants-unread");
    let entry = dir.join("main.nvs");
    std::fs::write(&entry, "<?nvs\n").expect("the entry file is written");
    std::fs::write(dir.join("nvs.toml"), "[capabilities.net]\nconect = true\n")
        .expect("the configuration is written");

    assert_eq!(
        of_program(&entry, None),
        "The grants of this program are in `nvs.toml`. This configuration does not load, \
         and `nvs check` shows the error."
    );
}
