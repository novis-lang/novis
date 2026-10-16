// Completion inside a file named `nvs.toml`: the keys of the block the cursor is in, and the block
// headers after a `[` (`rule:ide/the-extension-claims-nvs-only`).
//
// This is the one provider the extension registers outside the `nvs` language, and it claims nothing:
// no language id, no file extension, no grammar. The selector is a file name, so a TOML extension
// keeps the document and the editor merges its completion with this one. What is offered is the
// server's answer to `nvs/directives`, read from the default file the binary ships, and this file only
// converts positions and asks. The request and its answer are spelled in
// `crates/nvs-lsp/src/directives.rs`.

import {
  CancellationToken,
  CompletionItem,
  CompletionItemProvider,
  ExtensionContext,
  languages,
  Position,
  TextDocument,
} from "vscode";
import { LanguageClient, CompletionItem as WireItem } from "vscode-languageclient/node";

// The server's request, spelled where `crates/nvs-lsp/src/directives.rs` spells it.
const METHOD = "nvs/directives";

// The files this provider is offered for: any file named `nvs.toml`, on disk.
export const SELECTOR = { scheme: "file", pattern: "**/nvs.toml" };

let serving: LanguageClient | undefined;

/** Register the provider. Called once from `activate`; it answers nothing until a server runs. */
export function install(context: ExtensionContext): void {
  context.subscriptions.push(languages.registerCompletionItemProvider(SELECTOR, new Provider(), "["));
}

/** The client every ask goes to, or `undefined` while none is running. */
export function serve(client: LanguageClient | undefined): void {
  serving = client;
}

class Provider implements CompletionItemProvider {
  async provideCompletionItems(
    document: TextDocument,
    position: Position,
    token: CancellationToken,
  ): Promise<CompletionItem[]> {
    const client = serving;
    if (client === undefined) {
      return [];
    }
    let answered: WireItem[];
    try {
      answered = await client.sendRequest<WireItem[]>(
        METHOD,
        { text: document.getText(), position: client.code2ProtocolConverter.asPosition(position) },
        token,
      );
    } catch {
      // A server that cannot answer leaves the TOML extension's own completion, which is what the
      // file had before this provider existed.
      return [];
    }
    return client.protocol2CodeConverter.asCompletionResult(answered, undefined, token);
  }
}
