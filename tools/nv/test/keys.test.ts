import { describe, expect, test } from "bun:test";
import { analyse } from "../keys/scan.ts";

const BASE = `//! A module.
use std::fmt;

/// Says one.
pub fn one() -> i32 { 1 }

const CARD: MethodDoc = MethodDoc { short: "Returns one.", params: &[ParamDoc { name: "x" }] };
const ROWS: &[CaseDoc] = &[CaseDoc { name: "A" }];
const LIMIT: usize = 4;

#[cfg(test)]
mod tests {
    #[test]
    fn it() { assert_eq!(super::one(), 1); let _ = include_str!("fixture.txt"); }
}

static DATA: &str = include_str!("data.txt");
`;

function tiersOf(text: string) {
  const a = analyse(text);
  return { docs: a.docs, code: a.code, shipped: a.shipped, card: a.card };
}

/** Which tiers moved between two texts. */
function moved(a: string, b: string): string[] {
  const x = tiersOf(a);
  const y = tiersOf(b);
  return (Object.keys(x) as (keyof typeof x)[]).filter((k) => x[k] !== y[k]);
}

describe("the tiers", () => {
  test("a plain comment and a layout change move no tier", () => {
    expect(moved(BASE, BASE.replace("use std::fmt;", "use std::fmt; // why"))).toEqual([]);
    expect(moved(BASE, BASE.replace("pub fn one() -> i32 { 1 }", "pub fn one()\n    -> i32 {\n    1\n}"))).toEqual([]);
  });

  test("a doc comment moves the docs tier alone", () => {
    expect(moved(BASE, BASE.replace("Says one.", "Says the number one."))).toEqual(["docs"]);
  });

  test("an edit inside the test module moves docs and code, and neither shipped nor card", () => {
    expect(moved(BASE, BASE.replace("fn it() {", "fn it() { let _two = 2;"))).toEqual(["docs", "code"]);
  });

  test("a card's text moves every tier but card, a slice of cards too", () => {
    expect(moved(BASE, BASE.replace("Returns one.", "Gives one."))).toEqual(["docs", "code", "shipped"]);
    expect(moved(BASE, BASE.replace('name: "A"', 'name: "B"'))).toEqual(["docs", "code", "shipped"]);
  });

  test("a const that is not a card moves the card tier", () => {
    expect(moved(BASE, BASE.replace("LIMIT: usize = 4", "LIMIT: usize = 5"))).toEqual(["docs", "code", "shipped", "card"]);
  });

  test("tokens that layout would join never share a key", () => {
    expect(moved("fn f() { a & &b }", "fn f() { a &&b }")).toContain("code");
  });

  test("an include site knows whether it is in a test module", () => {
    expect(analyse(BASE).includes).toEqual([
      { path: "fixture.txt", inTest: true },
      { path: "data.txt", inTest: false },
    ]);
  });
});
