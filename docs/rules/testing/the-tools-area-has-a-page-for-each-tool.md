The website has a Tools area, the fourth after Guides, Syntax and Reference and before In-Depth, with
an overview page and one page for each developer tool: the debug viewer, the debugger, the profiler,
the editor, the formatter, the test runner, the API document and the CLI. The overview shows each tool
with one picture and one line and links to its page. Each tool is a directory under `tools/`, its page
the directory's `index.mdx`, and a tool with more than one page keeps the rest beside it, as the test
runner keeps coverage.

Every picture on these pages is generated from a real run and checked, never drawn by hand: a terminal
block (`rule:testing/a-terminal-block-shows-what-nvs-printed`), an editor replay
(`rule:testing/a-replay-plays-what-the-language-server-returned`), a debugger replay
(`rule:testing/a-debugger-replay-plays-what-nvs-dap-returned`) or a screenshot
(`rule:testing/a-screenshot-is-taken-from-a-real-run`). A picture nothing checks shows a tool as it
was, and nothing notices when it changes.

**A page's place is its subject.** A page about using one of these tools lives under Tools. A page
about writing Novis code, or about running a program in production, stays where it is: a guide to
testing a route writes code, and a guide to running a server as a service runs a program. The user
decided that a page that moves into Tools leaves nothing behind: no stub page, no note that it moved,
no link back and no redirect. A list of guides that named it drops the entry. Every other link to its
old address is changed to the new one, and its search questions are keyed to the new address. A reader
who arrives from a search lands on the page that answers, and the old address is not one the site
builds.

`bun nv site --check structure` holds the five areas and the sidebar, and its `tools` part holds the
eight pages, each with a checked picture and a `covers:` list
(`rule:testing/website-pages-are-stamped`), and the overview's section for each, whose `## ` heading
links the page and which shows a checked picture. The home page's Tooling section links each `nvs`
command to its tool page, and the overview.

- **Memory:** nothing in the language or the binary. The site carries the pictures' files, and a page
  loads only its own.

**Declined:**
- **Tool pages kept under Guides.** A guide shows how to write a program, and a reader looking for
  what the debugger does would not look there.
- **A redirect or a stub at a moved page's old address.** A moved page leaves no trace, and the site
  has no redirects (`bun nv site --check structure`).
