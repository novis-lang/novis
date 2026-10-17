//! `rule:http-server/a-jit-page-is-never-writable-and-executable-at-once`, read back from the OS:
//! a page Novis publishes as code is executable and not writable.
//!
//! The loaded arm, a cached payload placed and relocated by `nvs-cli`'s loader, is
//! `crates/nvs-cli/src/cache.rs`'s test of the same name. Both arms end in
//! [`nvs_codegen::make_executable`]'s sequence, and the last test here holds that function to it
//! on its own.

mod common;

use region::Protection;

/// Asserts that the page under `address` is executable and not writable, as the OS reports it.
fn assert_code_page(address: *const u8, what: &str) {
    let page = region::query(address).expect("the address is inside a mapping of this process");
    let protection = page.protection();
    assert!(
        protection.contains(Protection::EXECUTE),
        "{what}: the page is not executable ({protection:?})"
    );
    assert!(
        !protection.contains(Protection::WRITE),
        "{what}: the page is writable and executable at once ({protection:?})"
    );
}

#[test]
fn a_compiled_unit_runs_from_pages_that_are_not_writable() {
    let unit = common::compile(
        "<?nvs\nclass Greeter {\n    public static function hello(): string { return \"hi\"; }\n}\n\
         echo Greeter::hello();\n",
    )
    .expect("the fixture compiles");
    for name in [nvs_ir::lower::ENTRY_SCRIPT_LABEL, "Greeter::hello"] {
        let code = unit
            .raw_function(name)
            .expect("the fixture defines the function");
        assert_code_page(code as *const u8, name);
    }
}

#[test]
fn make_executable_leaves_no_writable_view() {
    let mut pages = memmap2::MmapMut::map_anon(4096).expect("an anonymous page");
    pages[0] = 0xC3;
    let code = nvs_codegen::make_executable(pages).expect("the platform publishes the page");
    assert_code_page(code.as_ptr(), "make_executable");
}
