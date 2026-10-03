// What `package.json` claims, asserted rather than reviewed.
//
// Every identifier here is public API: a setting name lives in somebody's `settings.json` and a
// command id in their keybindings, so `rule:ide/contributions-are-frozen-and-only-ever-added`
// freezes the roster and this file is where it is frozen. The lists below are exhaustive on
// purpose — a later milestone adds a name to them, and nothing ever renames one.
//
// A roster is a promise, so this file also asserts that each promise is kept: every contributed
// command reaches a `registerCommand` call, and every contributed setting reaches a reader — the
// client's own, or the server's out of the section the client forwards. Neither is visible from the
// manifest, and a user meets the gap as *command not found* in the palette or as a settings entry
// that changes nothing. An identifier is never removed to make one of these pass
// (`rule:ide/contributions-are-frozen-and-only-ever-added`): it is answered.
//
// The other three claims break a user's editor quietly rather than loudly, which is why they are
// tests: the extension claims `.nvs` and never `.php`
// (`rule:ide/the-extension-claims-nvs-only`), its runtime dependencies are on an allowlist so a
// second implementation of the language cannot arrive as one
// (`rule:ide/dependencies-are-allowlisted`), and it contributes no colour
// (`rule:ide/novis-ships-names-not-colours`).

import * as assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const ROOT = resolve(__dirname, "..", "..", "..");

interface ConfigProperty {
  type: string;
  default?: unknown;
  enum?: string[];
}

interface Manifest {
  name: string;
  displayName: string;
  publisher: string;
  license: string;
  icon: string;
  homepage: string;
  repository: { url: string; directory: string };
  engines: Record<string, string>;
  extensionKind: string[];
  activationEvents: string[];
  main: string;
  version: string;
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  scripts: Record<string, string>;
  contributes: {
    languages: {
      id: string;
      extensions: string[];
      configuration?: string;
      icon?: { light: string; dark: string };
    }[];
    configuration: { title: string; properties: Record<string, ConfigProperty> };
    commands: { command: string; title: string; category?: string }[];
    icons?: Record<string, {
      description: string;
      default: { fontPath: string; fontCharacter: string };
    }>;
    configurationDefaults?: Record<string, unknown>;
    themes?: unknown[];
  };
}

interface LanguageConfiguration {
  comments: { lineComment: string; blockComment: [string, string] };
  brackets: [string, string][];
  autoClosingPairs: { open: string; close: string; notIn?: string[] }[];
  surroundingPairs: [string, string][];
  wordPattern: string;
  indentationRules: { increaseIndentPattern: string; decreaseIndentPattern: string };
  onEnterRules: { beforeText: string; action: { indent: string; appendText?: string } }[];
  folding: { markers: { start: string; end: string } };
}

const manifestText = readFileSync(join(ROOT, "package.json"), "utf8");
const manifest = JSON.parse(manifestText) as Manifest;

// The roster, frozen. Added to by a later milestone, never renamed.
const SETTINGS = [
  "nvs.path",
  "nvs.lsp.enable",
  "nvs.lsp.debounce",
  "nvs.lsp.trace.server",
  "nvs.check.scope",
  "nvs.codeLens.enable",
  "nvs.template.services",
  "nvs.template.format",
  "nvs.secrets.redact",
  "nvs.taint.mark",
  "nvs.completion.phpNames",
  "nvs.stubs.dir",
];

const COMMANDS = [
  "nvs.run",
  "nvs.test",
  "nvs.showAst",
  "nvs.restartServer",
  "nvs.revealSecret",
  "nvs.hideSecrets",
  "nvs.convertToHtmlLiteral",
  "nvs.checkWorkspace",
  "nvs.downloadBinary",
  "nvs.openReleases",
];

// `rule:ide/dependencies-are-allowlisted` is over runtime dependencies: a test library ships to
// nobody, a language implementation arriving here ships to everyone.
const ALLOWED_DEPENDENCIES = ["vscode-languageclient"];

// The client's own TypeScript, as one text. Nothing here can run `activate` — the headless tier has
// no `vscode` module to import (`scripts/headless.mjs`) — so what the two answering tests below
// assert is the code that answers an identifier rather than the effect of running it. Every file
// under `src/` counts and not `extension.ts` alone: a command a module registers on `activate`'s
// behalf is still answered, and what must not appear twice is the roster, not the call.
const CLIENT = readdirSync(join(ROOT, "src"))
  .filter((file) => file.endsWith(".ts"))
  .map((file) => readFileSync(join(ROOT, "src", file), "utf8"))
  .join("\n");

// The other side of the wire, which is the other place a setting is read.
// `Settings::from_initialize` walks the `nvs` section by key path, so `nvs.completion.phpNames` is
// read at `&["completion", "phpNames"]` there and by nothing in this package. Whitespace is
// collapsed because the assertion is about the path, not about where rustfmt wrapped it.
const SERVER = readFileSync(
  join(ROOT, "..", "..", "crates", "nvs-lsp", "src", "settings.rs"),
  "utf8",
).replace(/\s+/g, " ");

/** Whether the client registers `id`, which is the whole of what answers a palette entry. */
function registered(id: string): boolean {
  return new RegExp(`registerCommand\\(\\s*"${escaped(id)}"`).test(CLIENT);
}

/**
 * Whether the client itself reads `key` — by its whole name, or by the tail
 * `workspace.getConfiguration("nvs")` takes.
 *
 * `nvs.lsp.trace.server` is read by `vscode-languageclient` and not by any line of this package:
 * it takes `<id>.trace.server` from the id the `LanguageClient` was constructed with, so that id is
 * the evidence, and renaming it is what would make the setting inert.
 */
function read(key: string): boolean {
  const tail = key.slice("nvs.".length);
  const traced = /new LanguageClient\(\s*"([^"]+)"/.exec(CLIENT)?.[1];
  return CLIENT.includes(`"${key}"`)
    || new RegExp(`\\.get(<[^>]*>)?\\(\\s*"${escaped(tail)}"`).test(CLIENT)
    || key === `${traced}.trace.server`;
}

/**
 * Whether the server reads `key` out of a section this client forwards.
 *
 * Both halves are required, because either alone is inert: a key the server reads that no client
 * sends leaves the default in force, and a section forwarded to a server that reads nothing out of
 * it changes nothing either. The client sends the section as it holds it, nested, which is why the
 * key path here is the setting's name with `nvs` taken off the front.
 */
function forwarded(key: string): boolean {
  const path = key.split(".").slice(1).map((part) => `"${part}"`).join(", ");
  return CLIENT.includes("initializationOptions") && SERVER.includes(`&[${path}]`);
}

/** `name` as a regular expression that matches it literally, its dots included. */
function escaped(name: string): string {
  return name.split(".").join("\\.");
}

describe("the extension's identity", () => {
  it("is novis-lang.nvs, a workspace extension", () => {
    // The publisher is the GitHub organisation, which is the only spelling of the project's
    // owner anywhere in the tree; the package name is the binary's. The two together are what
    // the editor shows under the title and what a Marketplace URL would be built from.
    //
    // It is an identifier and not a name: `vsce` checks it against `/^[a-z0-9][a-z0-9-]*$/i`,
    // so the website's `novis-lang.org` cannot go here — a dot would also make `publisher.name`
    // ambiguous. The domain is the `homepage` instead.
    assert.equal(manifest.publisher, "novis-lang");
    assert.equal(manifest.name, "nvs");
    assert.ok(/^[a-z0-9][a-z0-9-]*$/i.test(manifest.publisher),
              "the publisher is not a Marketplace identifier");
    assert.deepEqual(manifest.extensionKind, ["workspace"]);
  });

  it("points at the repository and the website, not at a name either has outgrown", () => {
    // Both URLs said `nvs-lang/nvs` until the move to `novis-lang/novis`, and neither is fetched
    // by anything that would have failed — a Marketplace listing would simply have carried two
    // dead links. `docs/release.md` § 1 records the same staleness in `Cargo.toml`.
    assert.equal(manifest.repository.url, "https://github.com/novis-lang/novis.git");
    assert.equal(manifest.repository.directory, "editors/vscode");
    assert.equal(manifest.homepage, "https://novis-lang.org");
  });

  it("takes its title from the README's H1", () => {
    // `docs/decisions/0151.md` § 3 gave the project's title one home and had `nvs --help` read
    // from it; the editor's title for the extension is the same fact reaching a third surface,
    // so it is asserted against that home rather than agreed with it by hand.
    const readme = readFileSync(join(ROOT, "..", "..", "README.md"), "utf8");
    assert.equal(manifest.displayName, /^# (.+)$/m.exec(readme)?.[1]);
  });

  it("is versioned with the binary it speaks to", () => {
    // The extension and `nvs` are released from one repository at one version, which is what lets
    // the client refuse a server by comparing against its own
    // (`rule:ide/the-extension-refuses-a-binary-it-does-not-understand`, `src/version.ts`). The
    // workspace's version is in the root `Cargo.toml`.
    const cargo = readFileSync(join(ROOT, "..", "..", "Cargo.toml"), "utf8");
    const workspace = /^version = "([^"]+)"$/m.exec(cargo)?.[1];
    assert.equal(manifest.version, workspace);
  });

  it("ships the Novis logo as its icon", () => {
    // The one asset in the `.vsix`, and the only place the extension is branded. It is a copy of
    // `website/media/novis-logo.png` rather than a reference to it: an extension package carries
    // no path out of its own directory. A `.vscodeignore` entry that swept `media/` away would
    // leave the manifest naming a file the package does not hold, which `vsce package` refuses,
    // so the existence check here is what fails first and says why.
    assert.equal(manifest.icon, "media/novis-logo.png");
    const icon = readFileSync(join(ROOT, manifest.icon));
    assert.deepEqual([...icon.subarray(0, 8)], [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a],
                     "the icon is not a PNG");
    // IHDR is the first chunk: width and height are the two big-endian words at byte 16.
    assert.ok(icon.readUInt32BE(16) >= 128 && icon.readUInt32BE(20) >= 128,
              "the Marketplace wants at least 128x128");
  });

  it("ships the repository's own license", () => {
    // The extension is licensed as Novis is. `vsce package` wants a LICENSE inside the package
    // and a package carries no path out of its own directory, so the `package` script copies the
    // root file in before packaging, and the copy is git-ignored rather than a second home.
    const license = readFileSync(join(ROOT, "..", "..", "LICENSE"), "utf8");
    assert.equal(manifest.license, /^(\S+) License$/m.exec(license)?.[1]);
    assert.match(manifest.scripts.package, /copyFileSync\('\.\.\/\.\.\/LICENSE', 'LICENSE'\)/);
  });

  it("carries the mark for the status item as a font glyph", () => {
    // A `LanguageStatusItem`'s `text` renders `$(name)` and no image path, so the mark reaches it
    // as a one-glyph font that the editor tints with the item's own severity colour
    // (`src/extension.ts`, `report`). The header check is what catches a half-written build: a
    // WOFF opens with `wOFF`, and the third word is the length the whole file should have.
    const icon = manifest.contributes.icons?.["novis-mark"];
    assert.equal(icon?.default.fontPath, "./media/novis-icons.woff");
    assert.equal(icon?.default.fontCharacter, "\\E001");
    const font = readFileSync(join(ROOT, icon?.default.fontPath ?? ""));
    assert.equal(font.subarray(0, 4).toString("latin1"), "wOFF", "the icon font is not a WOFF");
    assert.equal(font.readUInt32BE(8), font.length, "the icon font is truncated");
  });

  it("points main at the bundled client", () => {
    // `bundle` writes `src/extension.ts` and everything it imports, `vscode-languageclient`
    // included, into this one file, and `vsce` runs it through `vscode:prepublish`. The `.vsix`
    // then carries no `node_modules/`, and a manifest naming any other file installs an
    // extension that activates and does nothing.
    assert.equal(manifest.main, "./out/extension.js");
    assert.match(manifest.scripts.bundle, /--outfile=out\/extension\.js /);
    assert.equal(manifest.scripts["vscode:prepublish"], "npm run --silent bundle");
    assert.match(manifest.scripts.package, /vsce package --no-dependencies /);
  });

  it("carries the npm scripts the repository's tooling calls", () => {
    // `nv verify` runs `test:headless`; the loop's acceptance sweep runs `test:host` on every
    // iteration and `package` at stage 8.
    for (const script of ["compile", "bundle", "lint", "test:headless", "test:host", "package"]) {
      assert.ok(manifest.scripts[script], `package.json declares no ${script} script`);
    }
  });
});

describe("the file types the extension claims", () => {
  it("registers nvs as its own language, on .nvs alone", () => {
    const language = manifest.contributes.languages[0];
    assert.equal(language.id, "nvs");
    assert.deepEqual(language.extensions, [".nvs"]);
    assert.equal(language.configuration, "./language-configuration.json");
  });

  it("registers the case format as a second language, on .nvst and .lspt", () => {
    // `rule:ide/case-files-have-their-own-grammar`. One language for both formats: they share the
    // section shape, and the grammar reads either. It carries no language configuration — a case
    // has no comment or bracket of its own, and what is inside `--FILE--` is Novis's.
    assert.equal(manifest.contributes.languages.length, 2);
    const language = manifest.contributes.languages[1];
    assert.equal(language.id, "nvst");
    assert.deepEqual(language.extensions, [".nvst", ".lspt"]);
    assert.equal(language.configuration, undefined);
  });

  it("gives each language a file icon for each theme", () => {
    // What an explorer with a file-icon theme draws beside a `.nvs` or a `.nvst`. Four images,
    // because nothing tints these: the editor draws whichever of the pair matches the theme
    // exactly as it was authored, so `light` is the crimson tile, which holds against a light
    // ground, and `dark` the pink one. The `.nvst` pair is the same tile under a badge. They are
    // copies of `website/media/` as the Marketplace icon is — an extension package carries no
    // path out of its own directory. A theme that draws its own icon for a language wins, and
    // this is only the fallback.
    const icons: Record<string, { light: string; dark: string }> = {
      nvs: {
        light: "./media/novis-logo-file-icon-light-theme.svg",
        dark: "./media/novis-logo-file-icon-dark-theme.svg",
      },
      nvst: {
        light: "./media/novis-logo-file-icon-light-theme-nvst.svg",
        dark: "./media/novis-logo-file-icon-dark-theme-nvst.svg",
      },
    };
    for (const language of manifest.contributes.languages) {
      assert.deepEqual(language.icon, icons[language.id],
                       `${language.id} contributes the wrong file icon`);
      for (const path of Object.values(language.icon ?? {})) {
        assert.ok(readFileSync(join(ROOT, path), "utf8").includes("<svg"),
                  `${path} is not an SVG the package holds`);
      }
    }
  });

  it("activates on nvs and on nothing else", () => {
    // A grammar is contributed statically, so colouring a case starts nothing: the second language
    // adds no activation event, and opening one does not start the server.
    assert.deepEqual(manifest.activationEvents, ["onLanguage:nvs"]);
  });

  it("claims no php file type", () => {
    // Claiming `.php` would fight every PHP extension the user already has, and losing that
    // fight silently looks like Novis being broken. What is refused is the claim — a language id,
    // a file extension, an activation event, an embedded language — and not the three letters:
    // `nvs.completion.phpNames` is on the roster above and names PHP because the setting is about
    // PHP's names. `--ORACLE--`'s body is PHP and the case grammar includes `source.php` to colour
    // it, which is a reference to a grammar and not a claim on a file type — it stays in
    // `syntaxes/`, and no `embeddedLanguages` entry names php either.
    for (const language of manifest.contributes.languages) {
      assert.ok(["nvs", "nvst"].includes(language.id),
                `${language.id} is not one of Novis's own file types`);
      for (const extension of language.extensions ?? []) {
        assert.ok(!/php/i.test(extension), `${extension} is not Novis's to claim`);
      }
    }
    for (const claim of ['.php"', '"php"', ":php", "phtml"]) {
      assert.equal(manifestText.toLowerCase().includes(claim), false,
                   `the manifest claims ${claim}`);
    }
  });
});

describe("the frozen identifiers", () => {
  it("contributes exactly the settings the roster names", () => {
    assert.deepEqual(Object.keys(manifest.contributes.configuration.properties).sort(),
                     [...SETTINGS].sort());
  });

  it("contributes exactly the commands the roster names", () => {
    assert.deepEqual(manifest.contributes.commands.map((c) => c.command).sort(),
                     [...COMMANDS].sort());
  });

  it("registers every command it contributes", () => {
    // The roster test above passes whether or not a single command does anything, and this is the
    // one that does not: an id nothing registers is *command not found* from the palette. The list
    // is collected rather than asserted one at a time so the failure names every unanswered id at
    // once, which is the list of work rather than the first item of it.
    const unanswered = COMMANDS.filter((id) => !registered(id));
    assert.deepEqual(unanswered, [],
                     `${unanswered.join(", ")}: contributed, and no registerCommand answers it`);
  });

  it("reads or forwards every setting it contributes", () => {
    // A contributed setting that nothing reads is a control in the settings UI that moves nothing,
    // which is the same broken promise as an unregistered command and is quieter. Two consumers
    // count because a value's reader is not always in this package: the client reads one by name,
    // and `nvs-lsp` reads one out of the `initializationOptions` section the client forwards.
    const unread = SETTINGS.filter((key) => !read(key) && !forwarded(key));
    assert.deepEqual(unread, [], `${unread.join(", ")}: contributed, and nothing reads it`);
  });

  it("gives every command a title under one category", () => {
    for (const command of manifest.contributes.commands) {
      assert.ok(command.title, `${command.command} has no title`);
      assert.equal(command.category, "Novis");
    }
  });

  it("defaults each setting to what its rule says", () => {
    const properties = manifest.contributes.configuration.properties;
    assert.equal(properties["nvs.path"].default, "");
    assert.equal(properties["nvs.lsp.enable"].default, true);
    assert.equal(properties["nvs.lsp.debounce"].default, 150);
    assert.equal(properties["nvs.lsp.trace.server"].default, "off");
    assert.equal(properties["nvs.check.scope"].default, "workspace");
    assert.deepEqual(properties["nvs.check.scope"].enum, ["open", "workspace"]);
    assert.equal(properties["nvs.codeLens.enable"].default, true);
    assert.equal(properties["nvs.template.services"].default, true);
    assert.equal(properties["nvs.template.format"].default, true);
    assert.equal(properties["nvs.secrets.redact"].default, true);
    assert.equal(properties["nvs.taint.mark"].default, "off");
    assert.deepEqual(properties["nvs.taint.mark"].enum, ["off", "declaration", "sink"]);
    assert.equal(properties["nvs.completion.phpNames"].default, "all");
    assert.deepEqual(properties["nvs.completion.phpNames"].enum, ["all", "resolved", "off"]);
    assert.equal(properties["nvs.stubs.dir"].default, "");
  });
});

describe("what the extension may depend on", () => {
  it("depends only on the allowlist", () => {
    const dependencies = Object.keys(manifest.dependencies ?? {});
    for (const dependency of dependencies) {
      assert.ok(ALLOWED_DEPENDENCIES.includes(dependency),
                `${dependency} is not on the allowlist; language logic has one home, nvs-lsp`);
    }
  });

  it("keeps the tooling in devDependencies", () => {
    // A runtime dependency ships to users and a test library does not. `@vscode/test-electron`
    // downloads an editor, which is the clearest case of the two there is.
    for (const tool of ["typescript", "esbuild", "mocha", "eslint", "@vscode/test-electron"]) {
      assert.ok(manifest.devDependencies?.[tool], `${tool} is not a devDependency`);
      assert.equal(manifest.dependencies?.[tool], undefined);
    }
  });
});

describe("the colour the extension does not contribute", () => {
  it("ships no colour-customization default and no theme", () => {
    // The one default contributed is the paste preference for Novis documents, which names no
    // colour: it is what makes a paste apply its `use` lines rather than offer them in a widget
    // (`rule:ide/a-pasted-type-carries-its-use-line`).
    assert.deepEqual(manifest.contributes.configurationDefaults, {
      "[nvs]": { "editor.pasteAs.preferences": ["text.updateImports"] },
    });
    assert.equal(manifest.contributes.themes, undefined);
    for (const key of ["tokenColorCustomizations", "semanticTokenColorCustomizations", "colors"]) {
      assert.equal(manifestText.includes(key), false, `the manifest contributes ${key}`);
    }
  });
});

describe("language-configuration.json", () => {
  const configuration = JSON.parse(
    readFileSync(join(ROOT, "language-configuration.json"), "utf8"),
  ) as LanguageConfiguration;

  it("selects $total whole", () => {
    // Without the `$`, double-clicking `$total` selects `total` and every rename-adjacent
    // interaction is off by one character.
    const words = `$total = $x + 1;`.match(new RegExp(configuration.wordPattern, "g"));
    assert.deepEqual(words?.slice(0, 2), ["$total", "$x"]);
  });

  it("carries the comment, bracket and pair content", () => {
    assert.equal(configuration.comments.lineComment, "//");
    assert.deepEqual(configuration.comments.blockComment, ["/*", "*/"]);
    assert.equal(configuration.brackets.length, 3);
    assert.ok(configuration.autoClosingPairs.length >= 3);
    assert.ok(configuration.surroundingPairs.length >= 3);
    assert.ok(configuration.indentationRules.increaseIndentPattern);
    assert.ok(configuration.indentationRules.decreaseIndentPattern);
    assert.ok(configuration.folding.markers.start);
    assert.ok(configuration.folding.markers.end);
  });

  it("continues a /// run and continues no /** */ block", () => {
    // The entry a borrowed PHP configuration gets wrong: there `onEnterRules` continues a
    // `/** */` block, which in Novis is an ordinary comment nothing reads, and leaves the shape
    // the language does document with uncontinued.
    const continued = configuration.onEnterRules.filter((r) => r.action.appendText?.includes("///"));
    assert.equal(continued.length, 1);
    assert.ok(new RegExp(continued[0].beforeText).test("    /// a doc comment"));
    for (const rule of configuration.onEnterRules) {
      const appended = rule.action.appendText?.trim();
      assert.ok(appended === undefined || appended === "///",
                `an onEnterRule continues ${appended}, a shape Novis does not document with`);
    }
  });
});
