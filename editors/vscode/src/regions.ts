// The half of a `.nvs` file that is markup, answered by the editor's own services.
//
// `rule:ide/a-template-region-gets-services-but-no-second-formatter` is what this file implements.
// Inline HTML is the template engine (`rule:programs/first-party-framework`), so the markup in a
// `.nvs` file is where a web application's pages are written, and it gets the HTML service's
// completion, hover and colour picker rather than nothing. The boundaries come from the server as
// `nvs/regions` and the mapping from a boundary to a service is `template.ts`; what is here is the
// asking, the virtual document and the four registrations.
//
// **Nothing is registered as a formatting provider, and that absence is the rule's load-bearing
// half.** A `.nvs` file has one formatter, `nvs fmt`, and it is unconfigurable by decision
// (`rule:tooling/fmt-is-one-canonical-style`); a second one reflowing the markup inside it would
// make `nvs fmt --check` fail for a reason that is not "this file is laid out wrong". The server
// declares no formatting provider either (`crates/nvs-lsp/src/capabilities.rs`), so neither side of
// the wire offers one and `editor.formatOnSave` in a template runs `nvs fmt` over the whole file.
//
// **How a request is forwarded.** VS Code answers a service for a *document*, so the markup is
// handed to it as one: a virtual document under this file's own scheme, holding the regions where
// they are in the file and whitespace everywhere else. Positions therefore mean the same thing in
// both, which is why no range coming back needs mapping and why a hover lands on the byte the
// pointer is over. `template.ts`'s `virtual` is what builds it.
//
// **What forwarding does not reach.** A request is forwarded by asking the editor to run a provider
// over the virtual document, so what arrives is what a provider answers: completion, hover, the
// colour picker and the linked ranges that rename a tag. Two of the things the rule names are not
// providers and do not arrive this way. **Emmet** expands an abbreviation from the language of the
// document the cursor is in, so it would take `emmet.includeLanguages` mapping `nvs` to `html` —
// which turns abbreviation expansion on in the Novis half of the file as well, and is why nothing
// here contributes it. **Validation** is published per document by the HTML service for the
// documents it owns, and this virtual one is never opened in an editor for it to own. Both need a
// mechanism this file does not have, and are one item in `docs/agent/carried-gaps.md` rather than a
// half-registered provider here.
//
// **The regions are asked for per request rather than held.** A boundary is a lex
// (`crates/nvs-lsp/src/regions.rs` § *a lex, and not an analysis*), so the answer is cheap and one
// round trip buys certainty that the list describes the keystroke being answered — where a held
// list would have to be invalidated against every edit, and would forward into a region the user
// has since deleted. Nothing is drawn from this and nothing outlives the reply, which is what
// separates it from `redactions.ts`, where holding the last answer is the rule.

import {
  CancellationToken,
  Color,
  ColorInformation,
  ColorPresentation,
  CompletionContext,
  CompletionItem,
  CompletionList,
  EventEmitter,
  ExtensionContext,
  Hover,
  LinkedEditingRanges,
  Position,
  Range,
  TextDocument,
  TextDocumentContentProvider,
  Uri,
  commands,
  languages,
  workspace,
} from "vscode";
import { LanguageClient } from "vscode-languageclient/node";

import { Region, forwarded, service, virtual } from "./template";

// The server's own request, spelled where `crates/nvs-lsp/src/regions.rs` spells it. Its params are
// a `{textDocument, text?}` and its answer a list of `{range, language}`. The forwarding below asks
// about the open buffer and so sends no `text`; a caller with a text the buffer does not hold yet
// sends one and is answered about that instead.
const METHOD = "nvs/regions";

// The setting that turns the forwarding off, default `true`. A user with their own HTML tooling has
// to be able to get out of the way of ours, and `false` here forwards nothing anywhere rather than
// forwarding less: the decision is `template.ts`'s `forwarded`, which every provider below asks.
const SETTING = "nvs.template.services";

// The scheme the virtual documents live under. Nothing outside this file resolves one, and nothing
// is written anywhere — a `TextDocumentContentProvider` is asked for the text each time and what it
// answers is built from the buffer the user has open.
const SCHEME = "nvs-template";

// What the client claims, which is `extension.ts`'s `SELECTOR`: these providers answer for a Novis
// file and forward from inside it, and claim no HTML file anywhere.
const SELECTOR = [{ scheme: "file", language: "nvs" }];

// The text each virtual document currently holds, keyed by its own URI.
//
// Written immediately before a request is forwarded and read by the content provider, so the
// service sees exactly the document the forwarding decision was made against. The entry for a URI
// is replaced, never accumulated: there is one per open file with markup in it, and closing the
// editor drops it.
const shown = new Map<string, string>();
const changed = new EventEmitter<Uri>();

let serving: LanguageClient | undefined;

/**
 * Register the virtual documents and the providers that forward into them.
 *
 * Called once from `activate`, before any server is running. A provider asked while nothing is
 * answering forwards nothing, which is the same answer it gives outside a region.
 */
export function install(context: ExtensionContext): void {
  context.subscriptions.push(
    changed,
    workspace.registerTextDocumentContentProvider(SCHEME, content()),
    workspace.onDidCloseTextDocument((document) => forget(document)),
    // Completion, hover and the colour picker: the three the rule names that a service answers for
    // a position or for a document. Emmet and tag closing ride the same HTML service the first of
    // them reaches.
    //
    // `<` and `/` are trigger characters because a tag is what the user is opening or closing when
    // they matter; VS Code asks on a word character without being told.
    languages.registerCompletionItemProvider(SELECTOR, { provideCompletionItems: completing }, "<", "/"),
    languages.registerHoverProvider(SELECTOR, { provideHover: hovering }),
    // Renaming a tag: the editor keeps the two ends of an element in step from the ranges the HTML
    // service answers, and both address the file already.
    languages.registerLinkedEditingRangeProvider(SELECTOR, { provideLinkedEditingRanges: linked }),
    languages.registerColorProvider(SELECTOR, {
      provideDocumentColors: colours,
      provideColorPresentations: presentations,
    }),
  );
}

/**
 * The client every ask goes to, or `undefined` while none is running.
 *
 * A server that stops takes the embedded services with it, which is the direction that cannot be
 * wrong: without an answer there is no boundary, and forwarding on a guess is the second lexer
 * `rule:ide/a-template-region-gets-services-but-no-second-formatter` refuses.
 */
export function serve(client: LanguageClient | undefined): void {
  serving = client;
}

/** What a virtual document holds: whatever was put there for the ask now in flight. */
function content(): TextDocumentContentProvider {
  return {
    onDidChange: changed.event,
    provideTextDocumentContent: (uri: Uri): string => shown.get(uri.toString()) ?? "",
  };
}

/** Completion inside a region, from the service that owns it. */
async function completing(
  document: TextDocument,
  position: Position,
  _token: CancellationToken,
  context: CompletionContext,
): Promise<CompletionList | CompletionItem[] | undefined> {
  const uri = await embedded(document, position);
  if (uri === undefined) {
    return undefined;
  }
  return commands.executeCommand<CompletionList>(
    "vscode.executeCompletionItemProvider",
    uri,
    position,
    context.triggerCharacter,
  );
}

/** Hover inside a region, from the service that owns it. */
async function hovering(
  document: TextDocument,
  position: Position,
): Promise<Hover | undefined> {
  const uri = await embedded(document, position);
  if (uri === undefined) {
    return undefined;
  }
  const answered = await commands.executeCommand<Hover[]>(
    "vscode.executeHoverProvider",
    uri,
    position,
  );
  return answered?.[0];
}

/** The other end of the tag under the cursor, so renaming one renames both. */
async function linked(
  document: TextDocument,
  position: Position,
): Promise<LinkedEditingRanges | undefined> {
  const uri = await embedded(document, position);
  if (uri === undefined) {
    return undefined;
  }
  return commands.executeCommand<LinkedEditingRanges>(
    "vscode.executeLinkedEditingRangeProvider",
    uri,
    position,
  );
}

/**
 * Every colour the markup carries, for the editor's own picker.
 *
 * A document question rather than a position one, so the whole file is forwarded and the regions
 * are what the service can see in it. The ranges come back addressing the virtual document, which
 * holds every position where the file does — so they are the file's ranges already.
 */
async function colours(document: TextDocument): Promise<ColorInformation[] | undefined> {
  const uri = await embedded(document);
  if (uri === undefined) {
    return undefined;
  }
  return commands.executeCommand<ColorInformation[]>("vscode.executeDocumentColorProvider", uri);
}

/** How a colour the user picked is written, which is the service's spelling and not this one's. */
async function presentations(
  colour: Color,
  context: { document: TextDocument; range: Range },
): Promise<ColorPresentation[] | undefined> {
  const uri = await embedded(context.document, context.range.start);
  if (uri === undefined) {
    return undefined;
  }
  return commands.executeCommand<ColorPresentation[]>(
    "vscode.executeColorPresentationProvider",
    uri,
    colour,
    { uri, range: context.range },
  );
}

/**
 * The virtual document to forward `document` to, or nothing where no service answers.
 *
 * `position` is the cursor for a request that has one; without it the question is about the file,
 * and any region at all is enough to ask. Either way the answer is a fresh `nvs/regions`, and the
 * text is written down before the URI is handed out so what the service reads is what this decision
 * was made from.
 */
async function embedded(document: TextDocument, position?: Position): Promise<Uri | undefined> {
  // Scoped to the document rather than to the window, because a workspace folder with its own HTML
  // tooling is exactly the one that turns this off. The setting is read by its whole name, from the
  // root configuration, so the constant above is the only spelling of it in this package.
  const enabled = workspace
    .getConfiguration(undefined, document.uri)
    .get<boolean>(SETTING, true);
  const answered = await regions(document);
  const language = position === undefined
    ? first(enabled, answered)
    : forwarded(enabled, answered, position);
  if (language === undefined) {
    return undefined;
  }
  const uri = where(document, language);
  shown.set(uri.toString(), virtual(document.getText(), answered, language));
  changed.fire(uri);
  return uri;
}

/** The service a whole-document question goes to: the first region's, where there is one. */
function first(enabled: boolean, regions: readonly Region[]): string | undefined {
  if (!enabled) {
    return undefined;
  }
  for (const region of regions) {
    const language = service(region.language);
    if (language !== undefined) {
      return language;
    }
  }
  return undefined;
}

/** Where `document`'s markup is shown to `language`'s service. */
function where(document: TextDocument, language: string): Uri {
  return Uri.parse(
    `${SCHEME}://${language}/template.${language}?${encodeURIComponent(document.uri.toString())}`,
  );
}

/** Ask the server where `document` stops being Novis. */
async function regions(document: TextDocument): Promise<Region[]> {
  const client = serving;
  if (client === undefined || document.languageId !== "nvs") {
    return [];
  }
  try {
    return await client.sendRequest<Region[]>(METHOD, {
      textDocument: { uri: document.uri.toString() },
    });
  } catch {
    // No answer is no boundary, and no boundary forwards nothing: see `serve`.
    return [];
  }
}

/** Drop what was held for a document the editor has closed. */
function forget(document: TextDocument): void {
  for (const uri of shown.keys()) {
    if (uri.endsWith(`?${encodeURIComponent(document.uri.toString())}`)) {
      shown.delete(uri);
    }
  }
}
