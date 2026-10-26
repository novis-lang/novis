// The *View Profile* link the debug viewer writes, `vscode://novis-lang.nvs/profile?file=<path>`
// (`rule:testing/a-site-in-the-viewer-opens-in-the-editor`).
//
// Any web page can open such a link, so these cases pin what is accepted: one absolute speedscope
// file, and nothing else. `query` is given the way VS Code's `Uri.query` gives it, decoded once.

import * as assert from "node:assert/strict";

import { Link, SUFFIX, profileLink } from "../../src/profilelink";

function opens(link: Link): string {
  assert.ok("file" in link, `refused: ${"refused" in link ? link.refused : ""}`);
  return link.file;
}

function refused(link: Link): void {
  assert.ok("refused" in link, `accepted ${"file" in link ? link.file : ""}`);
}

describe("a profile link", () => {
  it("opens a POSIX path", () => {
    assert.equal(opens(profileLink("/profile", "file=/home/a/run.1234.speedscope.json")),
                 "/home/a/run.1234.speedscope.json");
  });

  it("opens a Windows path, with either slash", () => {
    assert.equal(opens(profileLink("/profile", "file=C:\\Users\\a\\run.speedscope.json")),
                 "C:\\Users\\a\\run.speedscope.json");
    assert.equal(opens(profileLink("/profile", "file=c:/Users/a/run.speedscope.json")),
                 "c:/Users/a/run.speedscope.json");
  });

  it("opens a percent-encoded path with spaces", () => {
    const path = "C:\\My Profiles\\a run.speedscope.json";
    assert.equal(opens(profileLink("/profile", `file=${encodeURIComponent(path)}`)), path);
    // The same link after VS Code has decoded it once.
    assert.equal(opens(profileLink("/profile", `file=${path}`)), path);
  });

  it("refuses any other path", () => {
    refused(profileLink("/open", "file=/home/a/run.speedscope.json"));
    refused(profileLink("/profile/", "file=/home/a/run.speedscope.json"));
    refused(profileLink("", "file=/home/a/run.speedscope.json"));
  });

  it("refuses a missing, an empty or a second file parameter", () => {
    refused(profileLink("/profile", ""));
    refused(profileLink("/profile", "file="));
    refused(profileLink("/profile", "file=/a/one.speedscope.json&file=/a/two.speedscope.json"));
    refused(profileLink("/profile", "path=/a/one.speedscope.json"));
    refused(profileLink("/profile", "file=/a/one.speedscope.json&line=3"));
  });

  it("refuses a relative path", () => {
    refused(profileLink("/profile", "file=run.speedscope.json"));
    refused(profileLink("/profile", "file=../run.speedscope.json"));
    refused(profileLink("/profile", "file=C:run.speedscope.json"));
  });

  it("refuses a network or device path", () => {
    refused(profileLink("/profile", "file=\\\\host\\share\\run.speedscope.json"));
    refused(profileLink("/profile", "file=//host/share/run.speedscope.json"));
    refused(profileLink("/profile", "file=\\\\?\\C:\\run.speedscope.json"));
  });

  it(`refuses a file that does not end in ${SUFFIX}`, () => {
    refused(profileLink("/profile", "file=/home/a/.ssh/id_ed25519"));
    refused(profileLink("/profile", "file=/home/a/run.json"));
    refused(profileLink("/profile", "file=C:\\a\\run.speedscope.json.exe"));
    refused(profileLink("/profile", `file=${encodeURIComponent("/a/run.speedscope.json\n.sh")}`));
  });
});
