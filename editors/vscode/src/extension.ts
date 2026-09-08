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
// server it drew from (`rule:security/redaction-ranges-come-from-the-server`).

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
  TransportKind,
} from "vscode-languageclient/node";

import * as redactions from "./redactions";
import { refusal } from "./version";

// The subcommand that is the server. `nvs lsp` speaks the protocol on its own stdin and stdout and
// writes nothing else there (`rule:ide/stdout-belongs-to-the-protocol`), which is why the transport
// needs no port, pipe or handshake of its own.
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

  // An empty `nvs.path` is a lookup on `PATH`: the command is the bare name, and the platform's
  // own resolution finds it or does not.
  const command = settings.get<string>("path", "").trim() || "nvs";
  const executable: Executable = { command, args: SUBCOMMAND, transport: TransportKind.stdio };
  const server: ServerOptions = { run: executable, debug: executable };
  const options: LanguageClientOptions = { documentSelector: SELECTOR, outputChannel: channel };

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
  report(`nvs lsp ${reported?.version}`, `${command} lsp is answering.`, LanguageStatusSeverity.Information);
}

async function stop(): Promise<void> {
  const running = client;
  client = undefined;
  // What is already concealed stays concealed while nothing is answering
  // (`rule:security/redaction-ranges-come-from-the-server`); this only says where to ask next.
  redactions.serve(undefined);
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

function report(text: string, detail: string, severity: LanguageStatusSeverity): void {
  if (status === undefined) {
    return;
  }
  status.text = text;
  status.detail = detail;
  status.severity = severity;
}

function reason(failure: unknown): string {
  return failure instanceof Error ? failure.message : String(failure);
}
