import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { abs } from "../lib/paths.ts";
import { declaredEnding, judgeHostile, type Ran } from "../proofs/run.ts";

const PATH = "tests/hostile/demo/01-three-steps.nvs";

/** An attack of three steps whose last one throws, as `rule:testing/hostile-case-contract` writes one. */
const attack = (marker: string) =>
  [
    "<?nvs",
    "// Attack: three steps, and the last one stops the program.",
    `// hostile: ${marker}`,
    "",
    "// 1. Catches a bad value.",
    "try { Core\\Str::repeat('x', -1); } catch (Throwable $e) { echo 'caught', \"\\n\"; }",
    "",
    "// 2. Reads a missing key.",
    "echo Core\\Arr::first([]), \"\\n\";",
    "",
    "// 3. Fills the memory.",
    'echo "step 3\\n";',
    "string $s = 'x';",
    "while (true) { $s = $s . $s; }",
  ].join("\n");

const report = (cls: string, line: number) =>
  JSON.stringify({ level: "error", msg: "it went wrong", fields: { class: cls }, nodes: [{ function: "f", file: "src/Other.nvs", line: 3 }, { function: "{main}", file: PATH, line }] });

const ran = (code: number, stdout: string, stderr: string): Ran => ({ code, stdout, stderr, timedOut: false, ms: 1 });
const judge = (marker: string, out: Ran) => judgeHostile(PATH, attack(marker), out, 10_000, false);

describe("an attack that ends early", () => {
  test("an attack that stops at the step and error class it declares passes", () => {
    expect(judge("ends-early 3 RuntimeError", ran(1, "caught\nstep 3\n", `${report("RuntimeError", 14)}\n`))).toEqual(["ok", ""]);
    expect(judge("ends-early 3 FATAL", ran(1, "caught\nstep 3\n", "FATAL: the request exceeded its memory limit\n"))).toEqual(["ok", ""]);
    expect(judge("ends-early 3 exit", ran(3, "caught\r\nstep 3\r\n", ""))).toEqual(["ok", ""]);
    // A hook that throws while the failure is reported writes a second report, and the first one ended the program.
    expect(judge("ends-early 3 RuntimeError", ran(1, "step 3\n", `${report("RuntimeError", 14)}\n${report("LogicError", 14)}\n`))).toEqual(["ok", ""]);
    // A response body written before the step may not end its line.
    expect(judge("ends-early 3 FATAL", ran(1, "caught\nxxxxstep 3\n", "FATAL: limit\n"))).toEqual(["ok", ""]);
  });

  test("an attack that stops at an earlier step fails and names both steps", () => {
    const [verdict, why] = judge("ends-early 3 RuntimeError", ran(1, "caught\n", `${report("RuntimeError", 9)}\n`));
    expect(verdict).toBe("fail");
    expect(why).toBe("stopped at step 2 with RuntimeError, declares step 3 with RuntimeError");
    // A `FATAL` names no line, so the step it stopped in is unknown.
    expect(judge("ends-early 3 FATAL", ran(1, "", "FATAL: the request exceeded its memory limit\n"))[1]).toBe(
      "stopped before step 3 with FATAL, declares step 3 with FATAL",
    );
  });

  test("an attack that stops with another error class fails and names both classes", () => {
    const [verdict, why] = judge("ends-early 3 FATAL", ran(1, "caught\nstep 3\n", `${report("LogicError", 14)}\n`));
    expect(verdict).toBe("fail");
    expect(why).toBe("stopped at step 3 with LogicError, declares step 3 with FATAL");
    expect(judge("ends-early 3 RuntimeError", ran(2, "step 3\n", ""))[1]).toBe("stopped at step 3 with exit, declares step 3 with RuntimeError");
  });

  test("an attack that runs to its last line while declaring an ending fails", () => {
    const [verdict, why] = judge("ends-early 3 FATAL", ran(0, "caught\nstep 3\n", ""));
    expect(verdict).toBe("fail");
    expect(why).toContain("ran to its last line");
  });

  test("a marker with no step, no ending or a step that is not the last fails before the run is judged", () => {
    const clean = ran(1, "step 3\n", "FATAL: limit\n");
    expect(judge("ends-early", clean)[1]).toContain("must name its step and its ending");
    expect(judge("ends-early 3", clean)[1]).toContain("must name its step and its ending");
    expect(judge("ends-early FATAL 3", clean)[1]).toContain("must name its step and its ending");
    expect(judge("ends-early 2 FATAL", clean)[1]).toBe("`ends-early` names step 2, which is not the file's last step, 3");
    // A file with no step comment is one attack, step 1.
    expect(declaredEnding("<?nvs\n// hostile: ends-early 1 exit\nexit(2);\n")).toEqual({ step: 1, ending: "exit" });
  });

  test("every attack in tests/hostile that ends early declares its step and error class", () => {
    const wrong: string[] = [];
    let seen = 0;
    const walk = (dir: string) => {
      for (const name of readdirSync(abs(dir))) {
        const path = join(dir, name).replace(/\\/g, "/");
        if (statSync(abs(path)).isDirectory()) walk(path);
        else if (path.endsWith(".nvs")) {
          const source = readFileSync(abs(path), "utf8");
          const declared = declaredEnding(source);
          if (declared === null) continue;
          seen++;
          if (typeof declared === "string") wrong.push(`${path}: ${declared}`);
          else if (!source.includes(`echo "step ${declared.step}\\n";`)) wrong.push(`${path}: step ${declared.step} never prints \`step ${declared.step}\``);
        }
      }
    };
    walk("tests/hostile");
    expect(seen).toBeGreaterThan(0);
    expect(wrong).toEqual([]);
  });
});
