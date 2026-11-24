// `bun nv names` on fixture signatures: one class per rule part, judged against the real tables, so a
// test fails when the tables stop backing the rule it names. No binary is read.

import { expect, test } from "bun:test";
import { type Class, type Member, fromSource, judge, loadTables, words } from "../cmd/names.ts";

const TABLES = loadTables();

const member = (name: string, returns: string, params = 0, kind: Member["kind"] = "instance", options?: string[]): Member => ({
  name,
  kind,
  returns,
  params,
  options,
});

/** The findings for one fixture class, as `member: part` lines. */
function verdict(c: Class, tables = TABLES): string[] {
  return judge([c], tables).map((f) => `${f.where.slice(c.name.length + 2)}: ${f.part}`);
}

test("a void member without an effect verb is named", () => {
  const c = { name: "Fixture\\Reply", members: [member("json", "void", 1), member("sendJson", "void", 1), member("onClose", "void", 1)] };
  expect(verdict(c)).toEqual(["json: part 1", "json: part 2"]);
});

test("a bool member without a predicate is named", () => {
  const c = {
    name: "Fixture\\Flags",
    members: [member("disabled", "bool"), member("isDisabled", "bool"), member("startsWith", "bool", 1, "static"), member("cancel", "bool")],
  };
  expect(verdict(c)).toEqual(["disabled: part 4"]);
});

test("a member returning a changed copy needs a copy name", () => {
  const tables = { ...TABLES, classes: { value: ["Fixture\\Picture"], changing: [] } };
  const c = {
    name: "Fixture\\Picture",
    members: [
      member("trim", "Picture"),
      member("resized", "Picture", 1),
      member("withFormat", "Picture", 1),
      member("flatten", "Picture"),
      member("abs", "Picture"),
    ],
  };
  expect(verdict(c, tables)).toEqual(["trim: part 3", "flatten: part 3"]);
});

test("a member changing its object in place needs an effect verb", () => {
  const tables = { ...TABLES, classes: { value: [], changing: ["Fixture\\Grid"] } };
  const c = { name: "Fixture\\Grid", members: [member("note", "Fixture\\Grid", 1), member("addNote", "Fixture\\Grid", 1)] };
  expect(verdict(c, tables)).toEqual(["note: part 3", "note: part 2"]);
});

test("a class returning itself must be classified", () => {
  const c = { name: "Fixture\\Builder", members: [member("addPart", "Builder", 1)] };
  const found = judge([c], TABLES);
  expect(found.map((f) => f.part)).toEqual(["part 3"]);
  expect(found[0]!.message).toContain("`value` or `changing`");
});

test("a truncated word is named", () => {
  const c = {
    name: "Fixture\\Files",
    judgeClassName: true,
    members: [member("makeDir", "void", 1, "static"), member("run", "void", 1, "static", ["cwd"]), member("max", "int", 2, "static"), member("maxLength", "uint")],
  };
  const found = judge([c], TABLES).map((f) => `${f.where}: ${f.part}: ${f.message}`);
  expect(found).toEqual([
    "Fixture\\Files::makeDir: full words: `dir` is a truncation: write directory",
    "Fixture\\Files::run option `cwd`: full words: `cwd` is a truncation: write workingDirectory",
    "Fixture\\Files::maxLength: full words: the math name `max` is allowed only as a whole member name",
  ]);
  expect(words("fromCwd")).toEqual(["from", "cwd"]);
});

test("a new noun member with an argument must be listed as a read", () => {
  const c = { name: "Fixture\\Book", members: [member("sheet", "Page", 1), member("addSheet", "Page", 1), member("title", "string")] };
  expect(verdict(c)).toEqual(["sheet: part 2"]);
  expect(verdict(c, { ...TABLES, reads: ["Fixture\\Book::sheet"] })).toEqual([]);
});

test("a package's public declarations are read from its source", () => {
  const src = [
    "<?nvs",
    "namespace Fixture\\Rest;",
    "final class Token",
    "{",
    "    public function constructor(",
    "        string $value,",
    "        int $expires,",
    "    ) {}",
    "    public static function of(array<string> $form, int $lifetime = 3600): Token",
    "    public function expired(): bool",
    "    private function secretPart(): string",
    "}",
  ].join("\n");
  const [c] = fromSource(src);
  expect(c!.name).toBe("Fixture\\Rest\\Token");
  expect(c!.members.map((m) => `${m.kind} ${m.name}/${m.params}: ${m.returns}`)).toEqual([
    "constructor constructor/2: void",
    "static of/2: Token",
    "instance expired/0: bool",
  ]);
});
