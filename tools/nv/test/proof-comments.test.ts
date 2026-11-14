import { describe, expect, test } from "bun:test";
import { commentProblemsOf } from "../proofs/collect.ts";

describe("the comment bounds of a proof program", () => {
  test("a doc comment's tag lines are each a sentence and do not count as lines", () => {
    const source = [
      "<?nvs",
      "// Shows a method with its tags.",
      "",
      "final class Payments {",
      "    /// Charges a card.",
      "    ///",
      "    /// @param $cents the amount to charge, in cents",
      "    /// @param $card the card number, written without spaces",
      "    /// @return the id of the charge",
      "    /// @throws CardDeclined when the amount is zero or less",
      "    public static function charge(int $cents, string $card): string {}",
      "}",
    ].join("\n");
    expect(commentProblemsOf(source)).toEqual([]);
  });

  test("a step's // line and the doc comment under it are two blocks", () => {
    const source = [
      "<?nvs",
      "// Attack: a tag names a parameter that is not there.",
      "",
      "final class Report {",
      "    // 1. Describes a parameter `$year` that the method does not have.",
      "    /// Builds the title.",
      "    /// It has two lines of prose.",
      "    public static function title(string $name): string {}",
      "}",
    ].join("\n");
    expect(commentProblemsOf(source)).toEqual([]);
  });

  test("the prose of a doc comment is still held to the line bound", () => {
    const source = [
      "<?nvs",
      "// Shows a long doc comment.",
      "",
      "final class Report {",
      "    /// One.",
      "    /// Two.",
      "    /// Three.",
      "    public function title(): string {}",
      "}",
    ].join("\n");
    expect(commentProblemsOf(source)).toEqual(["5: a comment above a step is 3 lines, and 2 is the bound"]);
  });

  test("a long tag sentence is still held to the word bound", () => {
    const long = Array.from({ length: 30 }, () => "word").join(" ");
    const source = ["<?nvs", "// Shows a tag.", "", "    /// Charges.", `    /// @return ${long}`, "    function f(): int {}"].join("\n");
    expect(commentProblemsOf(source)).toEqual(["4: a 31-word sentence opening `@return word word word word ...`; 25 is the bound"]);
  });
});
