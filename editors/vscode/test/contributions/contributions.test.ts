// What `package.json` claims, asserted rather than reviewed.
//
// Every identifier here is public API: a setting name lives in somebody's `settings.json` and a
// command id in their keybindings, so `rule:ide/contributions-are-frozen-and-only-ever-added`
// freezes the roster and this file is where it is frozen. The lists below are exhaustive on
// purpose — a later milestone adds a name to them, and nothing ever renames one.
//
// The other three claims break a user's editor quietly rather than loudly, which is why they are
// tests: the extension claims `.nvs` and never `.php`
// (`rule:ide/the-extension-claims-nvs-only`), its runtime dependencies are on an allowlist so a
// second implementation of the language cannot arrive as one
// (`rule:ide/dependencies-are-allowlisted`), and it contributes no colour
// (`rule:ide/novis-ships-names-not-colours`).

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
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
  "nvs.secrets.redact",
  "nvs.taint.mark",
];

const COMMANDS = [
  "nvs.run",
  "nvs.test",
  "nvs.showAst",
  "nvs.restartServer",
  "nvs.revealSecret",
  "nvs.hideSecrets",
];

// `rule:ide/dependencies-are-allowlisted` is over runtime dependencies: a test library ships to
// nobody, a language implementation arriving here ships to everyone.
const ALLOWED_DEPENDENCIES = ["vscode-languageclient"];

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

  it("points main at the compiled client", () => {
    // `tsc` puts `src/extension.ts` here, and a manifest naming anything else installs an
    // extension that activates and does nothing.
    assert.equal(manifest.main, "./out/src/extension.js");
  });

  it("carries the npm scripts the repository's tooling calls", () => {
    // `tools/verify.py` runs `test:headless`; the loop's stage 8 check runs `package`.
    for (const script of ["compile", "lint", "test:headless", "package"]) {
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

  it("gives both languages the logo as their file icon", () => {
    // What an explorer with a file-icon theme draws beside a `.nvs` or a `.nvst`. One image for
    // both themes: the logo is a transparent PNG that carries its own contrast, so a second
    // variant would be the same file under another name. A theme that draws its own icon for a
    // language wins, and this is only the fallback.
    for (const language of manifest.contributes.languages) {
      assert.deepEqual(language.icon,
                       { light: "./media/novis-logo.png", dark: "./media/novis-logo.png" },
                       `${language.id} contributes no file icon`);
    }
  });

  it("activates on nvs and on nothing else", () => {
    // A grammar is contributed statically, so colouring a case starts nothing: the second language
    // adds no activation event, and opening one does not start the server.
    assert.deepEqual(manifest.activationEvents, ["onLanguage:nvs"]);
  });

  it("names php nowhere in the manifest", () => {
    // Claiming `.php` would fight every PHP extension the user already has, and losing that
    // fight silently looks like Novis being broken. `--ORACLE--`'s body is PHP and the case grammar
    // includes `source.php` to colour it, which is a reference to a grammar and not a claim on a
    // file type — it stays in `syntaxes/`, and no `embeddedLanguages` entry names php either.
    assert.equal(/\bphp\b/i.test(manifestText), false, "the manifest mentions php");
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
    assert.equal(properties["nvs.secrets.redact"].default, true);
    assert.equal(properties["nvs.taint.mark"].default, "off");
    assert.deepEqual(properties["nvs.taint.mark"].enum, ["off", "declaration", "sink"]);
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
    // A runtime dependency ships to users and a test library does not.
    for (const tool of ["typescript", "mocha", "eslint"]) {
      assert.ok(manifest.devDependencies?.[tool], `${tool} is not a devDependency`);
      assert.equal(manifest.dependencies?.[tool], undefined);
    }
  });
});

describe("the colour the extension does not contribute", () => {
  it("ships no colour-customization default and no theme", () => {
    assert.equal(manifest.contributes.configurationDefaults, undefined);
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
