// What `initialize` declares, read off the real binary, and what this client does about it.
//
// The extension takes the server's word for every capability it uses, so the handshake is the one
// place a mismatch between the two halves is visible at all. The legend below is the clearest case:
// a semantic token names its type by index, so a server that reorders the list recolours every
// token in every document and no unit test on either side can see it
// (`crates/nvs-lsp/src/capabilities.rs`'s `TOKEN_TYPES` is the other end of this assertion).

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { refusal, series } from "../../src/version";
import { InitializeResult, Session } from "./session";

const ROOT = resolve(__dirname, "..", "..", "..");
const OWN_VERSION = (JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8")) as { version: string }).version;

// The wire order of the legend, which is an encoding and not a list: appending is the only safe
// edit to either of these.
const TOKEN_TYPES = [
  "namespace",
  "class",
  "interface",
  "enum",
  "enumMember",
  "type",
  "method",
  "property",
  "parameter",
  "variable",
  "typeParameter",
];
const TOKEN_MODIFIERS = ["defaultLibrary", "tainted", "secret"];

describe("the handshake with nvs lsp", function () {
  this.timeout(60_000);

  let session: Session;
  let declared: InitializeResult;

  before(async () => {
    ({ session, declared } = await Session.start());
  });

  after(() => {
    session?.kill();
  });

  it("is answered by a server that names itself and its version", () => {
    assert.equal(declared.serverInfo?.name, "nvs lsp");
    assert.ok(declared.serverInfo?.version, "the server reported no version at initialize");
    assert.ok(series(declared.serverInfo.version), `${declared.serverInfo.version} is not a release number`);
  });

  it("is answered by a version this extension understands", () => {
    // The two halves ship from one repository at one version, so this is red the day one is bumped
    // without the other — which is the point: in an editor the same mismatch is a refusal a user
    // has to read (`rule:ide/the-extension-refuses-a-binary-it-does-not-understand`).
    assert.equal(refusal(OWN_VERSION, declared.serverInfo?.version), undefined);
  });

  it("declares the legend the client decodes semantic tokens against", () => {
    const legend = declared.capabilities.semanticTokensProvider?.legend;
    assert.deepEqual(legend?.tokenTypes, TOKEN_TYPES);
    assert.deepEqual(legend?.tokenModifiers, TOKEN_MODIFIERS);
  });

  it("counts positions in utf-16, which is what an editor sends", () => {
    assert.equal(declared.capabilities.positionEncoding, "utf-16");
  });

  it("provides every request the client routes to it", () => {
    for (const capability of [
      "hoverProvider",
      "definitionProvider",
      "completionProvider",
      "documentSymbolProvider",
      "foldingRangeProvider",
      "selectionRangeProvider",
      "semanticTokensProvider",
      "documentLinkProvider",
      "codeActionProvider",
    ]) {
      assert.ok(declared.capabilities[capability], `the server declares no ${capability}`);
    }
  });
});

describe("the command line a language client spawns", function () {
  this.timeout(60_000);

  const started: Session[] = [];

  after(() => {
    for (const session of started) {
      session.kill();
    }
  });

  async function handshake(args: string[]): Promise<InitializeResult> {
    const { session, declared } = await Session.start(args);
    started.push(session);
    return declared;
  }

  // The flag no editor asks for and every client may send. `vscode-languageclient` reads an
  // `Executable`'s `transport` field as a claim about the *server's* argv and appends `--stdio`
  // from it, so the flag arrives from a setting that reads like transport configuration and is
  // written down nowhere in this repository's own launch. A server that refused it would exit
  // before the first frame, and every request the client routes would go with it while the syntax
  // grammar kept working — an editor that looks installed and answers nothing.
  it("is accepted with the transport flag a client appends unasked", async () => {
    const declared = await handshake(["--stdio"]);
    assert.equal(declared.serverInfo?.name, "nvs lsp");
  });

  // Accepted and *ignored*: it names the only transport there is, so it selects nothing. A future
  // flag that did select something would show up here as a different set of capabilities.
  it("declares the same server either way, because the flag chooses nothing", async () => {
    const plain = await handshake([]);
    const named = await handshake(["--stdio"]);
    assert.deepEqual(named.capabilities, plain.capabilities);
    assert.deepEqual(named.serverInfo, plain.serverInfo);
  });
});

describe("the version this client refuses", () => {
  it("refuses a server from another series, and says which one it wanted", () => {
    const refused = refusal("0.0.1", "9.9.9");
    assert.ok(refused?.includes("9.9.9"));
    assert.ok(refused?.includes("0.0.x"));
  });

  it("refuses a server that reports no version at all", () => {
    assert.ok(refusal("0.0.1", undefined));
  });

  it("refuses a version it cannot read as a release number", () => {
    assert.ok(refusal("0.0.1", "nightly"));
  });

  it("accepts a patch release either side of its own", () => {
    // A patch fixes answers; it does not change the shape of the protocol, and refusing one would
    // strand a user whose binary is one bugfix newer than their extension.
    assert.equal(refusal("0.0.1", "0.0.7"), undefined);
    assert.equal(refusal("0.0.7", "0.0.1"), undefined);
  });
});

describe("the way an editor ends a session", function () {
  this.timeout(60_000);

  it("leaves no process behind after shutdown and exit", async () => {
    // `nvs lsp` holds this process's stdin and stdout, so an editor that stops the client and gets
    // no exit leaks one server per session until something kills it. That is invisible to every
    // test that speaks the protocol without owning the process, which is why it is asserted here.
    const { session } = await Session.start();
    try {
      assert.equal(await session.close(), 0);
    } finally {
      session.kill();
    }
  });
});
