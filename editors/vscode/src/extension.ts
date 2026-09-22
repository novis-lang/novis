// The client: `nvs lsp` under `vscode-languageclient`, and the status item that says which binary
// answered.
//
// Everything an editor asks about the language is a request to that server
// (`rule:ide/one-server-two-thin-clients`), so this file spawns a process, reports what it found and
// decides nothing. There is no parser here, no formatter and no table of what a construct means —
// the contributions suite's dependency allowlist is what keeps it that way when a feature is added.
//
// Server health and version reach the user through a `LanguageStatusItem`, which is the API VS Code
// sanctions for it (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`); no status-bar
// item is hand-rolled here.
//
// The concealment of `secret` values is `redactions.ts`, installed once here and told which client
// to ask. It is on its own listeners rather than this file's, because what it draws outlives the
// server it drew from (`rule:ide/redaction-ranges-come-from-the-server`).
//
// `activate` returns a `Surface`: a read-only reading of the status item, the AST panel's tree and
// the ranges each editor was last decorated with, for the suite that runs inside the extension host.
// It is what this extension exports to every other one in the window, so nothing on it acts —
// `surface.ts` is where that is argued.

import {
  ConfigurationChangeEvent,
  ExtensionContext,
  LanguageStatusItem,
  LanguageStatusSeverity,
  OutputChannel,
  commands,
  languages,
  window,
  workspace,
} from "vscode";
import {
  ClientCapabilities,
  Executable,
  FeatureState,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  StaticFeature,
} from "vscode-languageclient/node";

import * as ast from "./ast";
import { binary, install as installCopies, runnable } from "./binary";
import * as format from "./format";
import * as imports from "./imports";
import * as redactions from "./redactions";
import * as regions from "./regions";
import { Stamp, Watch, stamp } from "./shadow";
import { stubsDirectory, withStubs } from "./stubs";
import { Health, Surface } from "./surface";
import * as tasks from "./tasks";
import * as testing from "./tests";
import { refusal } from "./version";

// The subcommand that is the server, and the whole of the command line. `nvs lsp` speaks the
// protocol on its own stdin and stdout and writes nothing else there
// (`rule:ide/stdout-belongs-to-the-protocol`), which is why the transport needs no port, pipe or
// handshake of its own.
//
// **`Executable.transport` is left unset on purpose, and it is not the same field it looks like.**
// The editor commands a completion item may name. An accepted class writes `Name::` and the list
// of its static members opens at once, and an accepted class after `new` writes `Name()` and
// signature help opens between the parentheses. Both are this editor's commands and not the
// server's, and the server names one only to a client that listed it here: another editor would
// answer an id it does not know with an error on every accepted item.
const EDITOR_COMMANDS = ["editor.action.triggerSuggest", "editor.action.triggerParameterHints"];

const editorCommands: StaticFeature = {
  fillClientCapabilities(capabilities: ClientCapabilities): void {
    capabilities.experimental = {
      ...(capabilities.experimental as object | undefined),
      commands: EDITOR_COMMANDS,
    };
  },
  initialize(): void {},
  getState(): FeatureState {
    return { kind: "static" };
  },
  clear(): void {},
};

// For an `Executable`, `vscode-languageclient` reads `TransportKind.stdio` as a claim about the
// server's *argv* and appends `--stdio` to this list before spawning; `nvs lsp` takes no such flag,
// so clap rejects it and the process exits before the first byte of protocol. Unset takes the
// identical stdio-pipe path in that client — same spawn, same reader and writer — and adds nothing
// to argv. Anything appended here has to be a flag the CLI actually accepts.
const SUBCOMMAND = ["lsp"];

// What the client claims: files the editor calls Novis, and nothing else. The case grammar's `nvst`
// language is coloured statically and starts no server, and `.php` is claimed by nobody here
// (`rule:ide/the-extension-claims-nvs-only`).
const SELECTOR = [{ scheme: "file", language: "nvs" }];

// The settings a running client is built from. A change to one of these restarts it, because
// neither reaches a server that is already spawned.
const RESPAWNING_SETTINGS = ["nvs.path", "nvs.lsp.enable"];

let client: LanguageClient | undefined;
let status: LanguageStatusItem | undefined;
let channel: OutputChannel | undefined;
let watching: Watch | undefined;
let turn: Promise<void> = Promise.resolve();

export async function activate(context: ExtensionContext): Promise<Surface> {
  channel = window.createOutputChannel("Novis");
  status = languages.createLanguageStatusItem("nvs.server", SELECTOR);
  status.name = "Novis";
  status.command = { command: "nvs.restartServer", title: "Restart" };
  context.subscriptions.push(
    channel,
    status,
    commands.registerCommand("nvs.restartServer", () => restart(context)),
    // Two entry points onto one execution path: each starts the Task `tasks.ts` builds, rather
    // than spawning a process of its own beside it (`rule:ide/tasks-carry-a-problem-matcher`).
    commands.registerCommand("nvs.run", () => tasks.execute("run")),
    commands.registerCommand("nvs.test", () => tasks.execute("test")),
    // The third way this extension reaches the binary, and the only one that reads what comes
    // back: `ast.ts` shells out to `nvs ast --json` and hands the document to the editor's own
    // tree view (`rule:ide/the-ast-panel-shells-out-to-the-cli`).
    commands.registerCommand("nvs.showAst", () => ast.show()),
    // The two halves of a reveal. They are registered here rather than in `redactions.ts` so the
    // command roster the manifest freezes has one place it is answered from.
    commands.registerCommand("nvs.revealSecret", (where?: Parameters<typeof redactions.reveal>[0]) =>
      redactions.reveal(where)),
    commands.registerCommand("nvs.hideSecrets", () => redactions.hide()),
    workspace.onDidChangeConfiguration((event: ConfigurationChangeEvent) => {
      if (RESPAWNING_SETTINGS.some((setting) => event.affectsConfiguration(setting))) {
        void restart(context);
      }
    }),
  );
  // First, because every spawn below and in the modules installed after it asks `binary.ts` which
  // file to run, and until this there is nowhere to keep a copy.
  installCopies(context);
  redactions.install(context);
  // The embedded services, likewise installed once and asking only while a server is answering.
  // The providers are registered from activation because a document with markup in it is often the
  // one the window opened on (`rule:ide/a-template-region-gets-the-editors-services-and-formatter`).
  regions.install(context);
  // The paste provider, registered from activation for the same reason: a copy made before the
  // server answers carries no imports, and the editor pastes it as it always did.
  imports.install(context);
  // The formatter, which is a process rather than a request: it starts `nvs fmt` and needs no
  // server, so it is installed here beside the rest and not in `start`.
  format.install(context);
  // The provider is what makes a hand-written `"type": "nvs"` entry in a `tasks.json` resolve;
  // `contributes.taskDefinitions` alone only describes the shape of one.
  tasks.install(context);
  // The view exists from activation rather than from the first `nvs.showAst`, because a tree view
  // is created once and the editor decides when to draw it; what the command does is fill it.
  ast.install(context);
  // The controller likewise exists from activation and holds nothing: discovery is a compile per
  // program, so the editor asks for it when the Testing view is opened rather than now.
  testing.install(context);
  await start(context);
  return surface();
}

/**
 * What this client exports, which is a reading of three things the editor's own API gives no way to
 * read back: `surface.ts` is where its shape, and the reason nothing on it acts, are written down.
 *
 * Each member is taken at the moment it is asked for, because this object is handed back once at
 * activation and everything it describes changes for the rest of the session.
 */
function surface(): Surface {
  return {
    get status(): Health | undefined {
      return status === undefined
        ? undefined
        : { text: status.text, detail: status.detail ?? "", severity: status.severity };
    },
    get ast() {
      return ast.tree();
    },
    get drawn() {
      return redactions.drawn();
    },
  };
}

export function deactivate(): Promise<void> {
  return stop();
}

// Spawn the server, and say in the status item what happened either way.
//
// The process is started from a copy of the binary and not from the binary, so the file the user
// named is free to be rebuilt or upgraded while the window is open, and a `Watch` on that file
// brings up the new build when it is — `shadow.ts` is where both halves are argued. The watch is set
// before anything is spawned and whatever the spawn comes to, because a build that would not start
// is exactly the one whose replacement should be picked up without being asked for.
//
// **A client that is already answering keeps answering until the next one is.** The copy is made
// and verified and the new server started and its version read while the old one still serves the
// editor, and only then is the old one stopped, so a rebuilt binary costs the user no gap in
// diagnostics or completion and nothing to look at: the status item is not even set to `starting`.
// What the old client is never allowed to do is outlive a new build that failed. A server that is
// answering is the build on disk or there is no server, so every way out of this function that does
// not end in a new client retires the old one and says why.
//
// The named binary itself is the fallback twice over: when no copy can be made, and when the copy
// will not start, which is what storage mounted `noexec` looks like. Either way the status item
// says the file is being held, since that is the thing the user will otherwise trip over later.
//
// The version is checked after `start` rather than before it because `initialize` is where a server
// reports one (`rule:ide/the-extension-refuses-a-binary-it-does-not-understand`) — there is nowhere
// earlier to read it from. What that rule refuses is a session, so a binary this client does not
// understand is stopped here and never handed a document, a request or the editor's attention.
async function start(context: ExtensionContext): Promise<void> {
  const previous = client;
  watching?.close();
  watching = undefined;
  const settings = workspace.getConfiguration("nvs");
  if (!settings.get<boolean>("lsp.enable", true)) {
    await retire(previous);
    report("off", "nvs.lsp.enable is false, so no server is running.", LanguageStatusSeverity.Information);
    return;
  }

  // Which file this is, is `binary.ts`'s to answer — every other spawn asks it the same question.
  const found = await runnable();
  const { shown, source } = found;
  let copied = found.copy;
  let held = found.held ?? "";
  if (source !== undefined) {
    watching = new Watch(source, copied?.stamp ?? (await stamp(source)), () => void restart(context));
  }

  if (previous === undefined) {
    report("starting", `${shown} lsp`, LanguageStatusSeverity.Information);
  }
  let starting: LanguageClient;
  try {
    starting = await launch(context, found.command);
  } catch (failure) {
    if (copied === undefined) {
      await retire(previous);
      report("not running", `${shown} lsp did not start: ${reason(failure)}`, LanguageStatusSeverity.Error);
      return;
    }
    held = `its copy did not start (${reason(failure)})`;
    copied = undefined;
    try {
      starting = await launch(context, binary());
    } catch (again) {
      await retire(previous);
      report("not running", `${shown} lsp did not start: ${reason(again)}`, LanguageStatusSeverity.Error);
      return;
    }
  }

  const reported = starting.initializeResult?.serverInfo;
  const refused = refusal(version(context), reported?.version);
  if (refused !== undefined) {
    await starting.stop();
    await retire(previous);
    report("wrong version", refused, LanguageStatusSeverity.Error);
    return;
  }

  client = starting;
  redactions.serve(client);
  regions.serve(client);
  imports.serve(client);
  await retire(previous);
  const detail = copied !== undefined
    ? `${shown} lsp is answering from a copy of the build of ${built(copied.stamp)}. The file may be replaced, and the new build takes over when it is.`
    : source !== undefined
      ? `${shown} lsp is answering from the file itself, which cannot be replaced while it runs: ${held}.`
      : `${shown} lsp is answering.`;
  report(`nvs lsp ${reported?.version}`, detail, LanguageStatusSeverity.Information);
}

// One client over `command`, started. It throws what `LanguageClient.start` throws, which is how a
// binary that cannot be spawned at all is told from one that answered.
async function launch(context: ExtensionContext, command: string): Promise<LanguageClient> {
  const executable: Executable = { command, args: SUBCOMMAND };
  const server: ServerOptions = { run: executable, debug: executable };
  // What the server is configured with. `crates/nvs-lsp/src/settings.rs` reads the `nvs` section out
  // of `initialize`'s `initializationOptions` and there is no `didChangeConfiguration` arm on the
  // other side, so this is the one moment a setting crosses the wire; a client that sent nothing —
  // which this one did until now — left every server-side setting at its default however the user
  // had configured it.
  //
  // The section goes over nested and whole, as the editor holds it, which is the shape that module
  // walks: no dotted keys are rebuilt here, and no list of names is kept here to be forgotten when
  // the server learns one. It is read from the root configuration rather than from `settings` above
  // because a `WorkspaceConfiguration` is a proxy with methods on it and this crosses a JSON-RPC
  // boundary, which wants a value.
  //
  // One key is filled in on the way: `nvs.stubs.dir`, where the server writes the `Core` stub tree
  // a jump opens. A user who named none gets a directory of this extension's own storage, per
  // server version, which is `stubs.ts`'s reasoning; the server takes whatever arrives and falls
  // back to a cache directory only for a client that sent nothing at all.
  //
  // The one middleware is an ordering and not a rewrite: nothing here changes a request, an answer
  // or a notification. `redactions.opened` asks what a document conceals, and it is called from
  // behind the `didOpen` it belongs to because the server answers nothing for a document it has not
  // been told about and this client holds that answer — `redactions.ts` § `opened` is why that
  // order is the difference between a concealed `secret` and one in cleartext.
  const options: LanguageClientOptions = {
    documentSelector: SELECTOR,
    outputChannel: channel,
    initializationOptions: withStubs(
      workspace.getConfiguration().get<object>("nvs"),
      stubsDirectory(context.globalStorageUri.fsPath, version(context)),
    ),
    middleware: {
      didOpen: async (document, next) => {
        await next(document);
        redactions.opened(document);
      },
    },
  };

  // The id is what the trace setting hangs off: `vscode-languageclient` reads `<id>.trace.server`,
  // which is the frozen `nvs.lsp.trace.server`.
  const starting = new LanguageClient("nvs.lsp", "Novis", server, options);
  starting.registerFeature(editorCommands);
  await starting.start();
  return starting;
}

// When the build a copy was taken from was written, in the user's own locale and clock: it is the
// one thing in the status item that tells two builds of one version apart.
function built(at: Stamp): string {
  return new Date(at.modified).toLocaleString();
}

async function stop(): Promise<void> {
  watching?.close();
  watching = undefined;
  await retire(client);
}

// Stop `previous`, which is either the client that was answering or nothing. When it is still the
// one the rest of the extension asks, they are told first that nobody is.
async function retire(previous: LanguageClient | undefined): Promise<void> {
  if (previous === undefined) {
    return;
  }
  if (client === previous) {
    client = undefined;
    // What is already concealed stays concealed while nothing is answering
    // (`rule:ide/redaction-ranges-come-from-the-server`); this only says where to ask next.
    redactions.serve(undefined);
    // The embedded services go the other way: without a server there is no boundary, and a client
    // that guessed one would be a second lexer.
    regions.serve(undefined);
    // A copy made while nothing answers carries no imports, and its paste is plain text.
    imports.serve(undefined);
  }
  try {
    await previous.stop();
  } catch {
    // A server that had already died cannot be stopped, and is as gone as one that could.
  }
}

// Restarts take turns. A changed setting and a replaced binary can ask within the same moment, and
// two interleaved would each retire the other's half-started client and leave a process nobody
// holds. There is no `stop` in one: `start` retires the client it found, once it has a successor.
function restart(context: ExtensionContext): Promise<void> {
  turn = turn.then(() => start(context));
  return turn;
}

// The extension's own version, which is the version of server it understands: the two are released
// from one repository together, and `version.ts` is where that comparison lives.
function version(context: ExtensionContext): string {
  return String(context.extension.packageJSON.version ?? "");
}

// Every state the item shows, marked. `novis-mark` is the manifest's `contributes.icons` entry, a
// one-glyph font rather than an image because `text` takes only `$(name)` — and being a glyph is
// what lets the editor tint it, so the mark turns red with the item on an `Error` severity without
// a second asset. Prefixed here so no call site can forget it.
function report(text: string, detail: string, severity: LanguageStatusSeverity): void {
  if (status === undefined) {
    return;
  }
  status.text = `$(novis-mark) ${text}`;
  status.detail = detail;
  status.severity = severity;
}

function reason(failure: unknown): string {
  return failure instanceof Error ? failure.message : String(failure);
}
