import { describe, expect, test } from "bun:test";
import { summaries } from "../cmd/verify.ts";
import { wayOut } from "../keys/escape.ts";

describe("wayOut", () => {
  test("a source that opens nothing outside its package has no way out", () => {
    expect(wayOut('fn t() { let s = std::fs::read_to_string("fixtures/a.txt"); }')).toBe("");
  });

  test("starting another program is a way out, and the package's own binary is not", () => {
    expect(wayOut('fn t() { Command::new("git"); }')).toBe("starts a process with `Command::new`");
    expect(wayOut('fn t() { Command::new(env!("CARGO_BIN_EXE_nvs")); }')).toBe("");
  });

  test("a literal that climbs out of its directory is a way out, and one rustc opens is not", () => {
    expect(wayOut('fn t() { read("../../docs/x.md"); }')).toContain("climbs out of its directory");
    expect(wayOut('const X: &str = include_str!("../../docs/x.md");')).toBe("");
  });

  test("a comment is never matched", () => {
    expect(wayOut('// Command::new("git") and "../../x"\nfn t() {}')).toBe("");
  });

  test("an escape sequence in a literal is not a path separator, and a raw literal's backslash is", () => {
    expect(wayOut('fn t() { let s = "..\\u{1f1e6}\\u{1f1f9}"; }')).toBe("");
    expect(wayOut('fn t() { read(r"..\\x.txt"); }')).toContain("climbs out of its directory");
  });

  test("a climbing literal listed as data is not a way out, and nothing else can be listed", () => {
    const data = new Set(['"../escape"']);
    expect(wayOut('fn t() { refuse("../escape"); }', data)).toBe("");
    expect(wayOut('fn t() { refuse("../escape"); read("../other"); }', data)).toContain('"../other"');
    expect(wayOut('fn t() { Path::new(env!("CARGO_MANIFEST_DIR")).join("../escape"); }', data)).toContain("CARGO_MANIFEST_DIR");
  });

  test("the manifest directory beside a climb is a way out", () => {
    expect(wayOut('fn t() { Path::new(env!("CARGO_MANIFEST_DIR")).parent(); }')).toContain("CARGO_MANIFEST_DIR");
  });
});

describe("summaries", () => {
  test("test sums every suite and counts the binaries not re-run", () => {
    const out = "     Unchanged a lib a\ntest result: ok. 3 passed; 0 failed\n     Running b\ntest result: ok. 2 passed; 1 failed";
    expect(summaries.test!(out)).toBe("5 passed, 1 failed  (2 suites, 1 binaries not re-run)");
  });

  test("fmt names the files it rewrote", () => {
    expect(summaries.fmt!("")).toBe("clean");
    expect(summaries.fmt!("D:\\mwl\\crates\\a\\src\\lib.rs\n")).toBe("formatted 1 file(s): lib.rs");
  });

  test("the case trees print their counts, and skipped only when there are some", () => {
    expect(summaries.cases!("10 passed, 0 failed, 0 skipped")).toBe("10 passed, 0 failed");
    expect(summaries.cases!("10 passed, 1 failed, 2 skipped")).toBe("10 passed, 1 failed, 2 skipped");
  });
});
