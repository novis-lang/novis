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
  Executable,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

import * as ast from "./ast";
import { binary } from "./binary";
import * as format from "./format";
import * as redactions from "./redactions";
import * as regions from "./regions";
import * as tasks from "./tasks";
import * as testing from "./tests";
import { refusal } from "./version";

// The subcommand that is the server, and the whole of the command line. `nvs lsp` speaks the
// protocol on its own stdin and stdout and writes nothing else there
// (`rule:ide/stdout-belongs-to-the-protocol`), which is why the transport needs no port, pipe or
// handshake of its own.
//
// **`Executable.transport` is left unset on purpose, and it is not the same field it looks like.**
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

export async function activate(context: ExtensionContext): Promise<void> {
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
  redactions.install(context);
  // The embedded services, likewise installed once and asking only while a server is answering.
  // The providers are registered from activation because a document with markup in it is often the
  // one the window opened on (`rule:ide/a-template-region-gets-services-but-no-second-formatter`).
  regions.install(context);
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
}

export function deactivate(): Promise<void> {
  return stop();
}

// Spawn the server, and say in the status item what happened either way.
//
// The version is checked after `start` rather than before it because `initialize` is where a server
// reports one (`rule:ide/the-extension-refuses-a-binary-it-does-not-understand`) — there is nowhere
// earlier to read it from. What that rule refuses is a session, so a binary this client does not
// understand is stopped here and never handed a document, a request or the editor's attention.
async function start(context: ExtensionContext): Promise<void> {
  const settings = workspace.getConfiguration("nvs");
  if (!settings.get<boolean>("lsp.enable", true)) {
    report("off", "nvs.lsp.enable is false, so no server is running.", LanguageStatusSeverity.Information);
    return;
  }

  // Which binary this is, is `binary.ts`'s to answer — the Tasks spawn the same one.
  const command = binary();
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
  const options: LanguageClientOptions = {
    documentSelector: SELECTOR,
    outputChannel: channel,
    initializationOptions: workspace.getConfiguration().get<object>("nvs"),
  };

  // The id is what the trace setting hangs off: `vscode-languageclient` reads `<id>.trace.server`,
  // which is the frozen `nvs.lsp.trace.server`.
  const starting = new LanguageClient("nvs.lsp", "Novis", server, options);
  report("starting", `${command} lsp`, LanguageStatusSeverity.Information);
  try {
    await starting.start();
  } catch (failure) {
    report("not running", `${command} lsp did not start: ${reason(failure)}`, LanguageStatusSeverity.Error);
    return;
  }

  const reported = starting.initializeResult?.serverInfo;
  const refused = refusal(version(context), reported?.version);
  if (refused !== undefined) {
    await starting.stop();
    report("wrong version", refused, LanguageStatusSeverity.Error);
    return;
  }

  client = starting;
  redactions.serve(client);
  regions.serve(client);
  report(`nvs lsp ${reported?.version}`, `${command} lsp is answering.`, LanguageStatusSeverity.Information);
}

async function stop(): Promise<void> {
  const running = client;
  client = undefined;
  // What is already concealed stays concealed while nothing is answering
  // (`rule:ide/redaction-ranges-come-from-the-server`); this only says where to ask next.
  redactions.serve(undefined);
  // The embedded services go the other way: without a server there is no boundary, and a client
  // that guessed one would be a second lexer.
  regions.serve(undefined);
  await running?.stop();
}

async function restart(context: ExtensionContext): Promise<void> {
  await stop();
  await start(context);
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
