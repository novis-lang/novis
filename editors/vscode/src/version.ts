// Which `nvs lsp` this client understands, and the sentence it refuses one with.
//
// `rule:ide/the-extension-refuses-a-binary-it-does-not-understand`: an older `nvs` earlier on `PATH`
// than the intended one is the likeliest support question this extension will ever get, and one
// comparison answers it out loud instead of producing confusing answers all session.
//
// The extension and the binary are released from one repository at one version, so the version the
// client understands is its own — there is no second constant here to go stale. The comparison is on
// the `major.minor` series because a patch release fixes answers rather than changing the protocol
// shape, and refusing one would strand a user whose binary is newer than their extension by a
// bugfix.
//
// Nothing in this file imports `vscode`. That is deliberate: the protocol suite runs it in plain
// Node against the version the real binary reports, which is the only place the two halves are
// checked against each other without an editor in the room.

// A release number, and nothing else: a pre-release or build suffix is matched but does not reach
// the series, so `0.1.0-rc.1` and `0.1.0` are the same one.
const RELEASE = /^(\d+)\.(\d+)\.(\d+)(?:[-+].*)?$/;

/** The `major.minor` series `version` belongs to, or `undefined` if it is not a release number. */
export function series(version: string): string | undefined {
  const parsed = RELEASE.exec(version.trim());
  return parsed === null ? undefined : `${parsed[1]}.${parsed[2]}`;
}

/**
 * Why this client will not talk to a server at `server`, or `undefined` if it will.
 *
 * `client` is the extension's own version, from its manifest. A server that reports no version at
 * all is refused too: `serverInfo.version` is optional in LSP, and a binary omitting it is either
 * not `nvs lsp` or is old enough to predate reporting one.
 */
export function refusal(client: string, server: string | undefined): string | undefined {
  const wanted = series(client);
  if (wanted === undefined) {
    return `This extension's own version, ${client}, is not a release number, so it cannot say which server it understands.`;
  }
  if (server === undefined) {
    return `The server reported no version at initialize, so it is not an nvs lsp this extension understands. It speaks to ${wanted}.x.`;
  }
  const found = series(server);
  if (found === undefined) {
    return `The server reported the version ${server}, which is not a release number. This extension speaks to ${wanted}.x.`;
  }
  if (found !== wanted) {
    return `The server is nvs ${server} and this extension speaks to ${wanted}.x. Point nvs.path at a matching binary, or install the extension that matches this one.`;
  }
  return undefined;
}
