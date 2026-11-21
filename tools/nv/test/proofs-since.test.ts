import { describe, expect, test } from "bun:test";
import type { Entry } from "../proofs/roster.ts";
import { classOf, novisDeclares, reached, rustNames } from "../proofs/since.ts";

const entry = (id: string, over: Partial<Entry> = {}): Entry => {
  const [group, member] = id.split("::");
  return {
    id,
    kind: "member",
    group: group!,
    path: `core/${group!.replace(/^Core\\/, "").replaceAll("\\", "-")}/${member}`,
    anchor: "",
    summary: "",
    help: "",
    ...over,
  };
};

const length = entry("Core\\Str::length", { anchor: "crates/nvs-stdlib/src/str.rs:223" });
const exitCode = entry("Core\\Test\\RunResult::exitCode");
const stdout = entry("Core\\Test\\RunResult::stdout");
const open = entry("Novis\\Image\\Image::open", { path: "novis/Image-Image/open" });
const roster = [length, exitCode, stdout, open];
const ids = (changed: string[], texts: Record<string, string> = {}) => reached(roster, changed, (p) => texts[p] ?? "").map((e) => e.id);

describe("the features a goal's work reaches", () => {
  test("a change to a feature's anchor file reaches it", () => {
    expect(ids(["crates/nvs-stdlib/src/str.rs"])).toEqual(["Core\\Str::length"]);
  });

  test("a new value class's members have no anchor, and the file that names the class reaches each of them", () => {
    const texts = { "crates/nvs-stdlib/src/test/run.rs": 'pub(crate) const RUN_RESULT_NAME: &str = r"Core\\Test\\RunResult";' };
    expect(ids(["crates/nvs-stdlib/src/test/run.rs"], texts)).toEqual(["Core\\Test\\RunResult::exitCode", "Core\\Test\\RunResult::stdout"]);
  });

  test("a change under a feature's example, attack or bench path reaches that feature alone", () => {
    expect(ids(["docs/examples/core/Test-RunResult/stdout/01-print.nvs"])).toEqual(["Core\\Test\\RunResult::stdout"]);
    expect(ids(["tests/hostile/core/Str/length/01-huge.nvs"])).toEqual(["Core\\Str::length"]);
    expect(ids(["benches/members/core/Str/length.nvs"])).toEqual(["Core\\Str::length"]);
  });

  test("a skip file reaches every feature of its group", () => {
    expect(ids(["data/proofs/skip/core/Test-RunResult/shared.json"])).toEqual(["Core\\Test\\RunResult::exitCode", "Core\\Test\\RunResult::stdout"]);
  });

  test("a component's Novis source reaches the members of the class it declares in its namespace", () => {
    const texts = { "extensions/image/nvs/Image.nvs": "<?nvs\nnamespace Novis\\Image;\n\nfinal class Image\n{\n}\n" };
    expect(ids(["extensions/image/nvs/Image.nvs"], texts)).toEqual(["Novis\\Image\\Image::open"]);
  });

  test("a change no feature lives in reaches nothing", () => {
    expect(ids(["README.md", "crates/nvs-lsp/src/index.rs"], { "crates/nvs-lsp/src/index.rs": "fn main() {}" })).toEqual([]);
  });
});

describe("the class a feature is declared in", () => {
  test("a member's group, and a class-scoped enum's class", () => {
    expect(classOf(exitCode)).toBe("Core\\Test\\RunResult");
    expect(classOf(entry("Novis\\Image\\Image::Fit", { kind: "enum", group: "types:enum" }))).toBe("Novis\\Image\\Image");
  });

  test("a Rust file names a class raw or escaped, and a longer name does not count", () => {
    expect(rustNames('r"Core\\Test\\RunResult"', "Core\\Test\\RunResult")).toBe(true);
    expect(rustNames('"Core\\\\Test\\\\RunResult"', "Core\\Test\\RunResult")).toBe(true);
    expect(rustNames('r"Core\\Test\\RunResultSet"', "Core\\Test\\RunResult")).toBe(false);
  });

  test("a Novis file declares a class only in its own namespace", () => {
    expect(novisDeclares("namespace Novis\\Pdf;\nclass Image {}", "Novis\\Image\\Image")).toBe(false);
    expect(novisDeclares("namespace Novis\\Image;\nenum Format {}", "Novis\\Image\\Format")).toBe(true);
  });
});
