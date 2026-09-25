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

  test("a function whose name ends in `current_dir` is not a read of the working directory", () => {
    expect(wayOut("#[test]\nfn drops_a_current_dir() { normalize(); }")).toBe("");
    expect(wayOut("fn t() { std::env::current_dir(); }")).toBe("reads the working directory");
  });

  test("product code may start a process, and test code in the same file may not", () => {
    const product = 'pub fn exec(p: &Path) { Command::new(p).spawn(); }\n';
    expect(wayOut(product, new Set(), "items")).toBe("");
    expect(wayOut(product, new Set(), "whole")).toBe("starts a process with `Command::new`");
    const tests = '#[cfg(test)]\nmod tests {\n  #[test]\n  fn t() { Command::new("git"); }\n}\n';
    expect(wayOut(product + tests, new Set(), "items")).toBe("starts a process with `Command::new`");
    expect(wayOut('#[cfg(all(test, unix))]\nfn helper() { Command::new("git"); }\n', new Set(), "items")).toBe("starts a process with `Command::new`");
    expect(wayOut('#![cfg(test)]\nfn helper() { Command::new("git"); }\n', new Set(), "items")).toBe("starts a process with `Command::new`");
  });

  test("product code may read the working directory, unless the same function climbs a path", () => {
    expect(wayOut("pub fn root() -> PathBuf { std::env::current_dir().unwrap() }", new Set(), "items")).toBe("");
    expect(wayOut("pub fn up() -> PathBuf { let d = std::env::current_dir().unwrap(); d.parent().unwrap().into() }", new Set(), "items")).toContain("climbs a path");
    expect(wayOut("pub fn root() { std::env::set_current_dir(\"x\"); }", new Set(), "items")).toBe("sets the working directory");
  });

  test("product code that climbs out of its directory is still a way out", () => {
    expect(wayOut('pub fn t() { read("../../docs/x.md"); }', new Set(), "items")).toContain("climbs out of its directory");
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
