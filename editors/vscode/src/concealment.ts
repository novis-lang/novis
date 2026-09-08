// What this window conceals, and which document each concealed range belongs to.
//
// The ranges arrive from the server and nothing here decides what a secret is
// (`rule:security/redaction-ranges-come-from-the-server`): this module holds the last answer for
// each document and says what is drawn now. A client that worked out for itself which literal
// carried `secret` would be a second implementation of the qualifier, which is what
// `rule:ide/one-server-two-thin-clients` refuses.
//
// Nothing here imports `vscode`, for the reason `src/version.ts` gives: the headless tier runs it in
// plain Node, and a decision whose test needs a display is a decision nobody tests.
//
// Two absences carry the rule between them. What is held is only ever *replaced* by an answer and
// never cleared by the lack of one — an error, a cancellation or a server restart leaves the
// concealment exactly as it is, because the failure mode of clearing it is the bytes becoming
// visible. And an answer of no ranges is an answer: `hold(uri, [])` conceals nothing, while never
// calling `hold` at all conceals whatever was concealed before.
//
// A reveal is a fact about this window and this range, held nowhere else
// (`rule:security/reveal-is-explicit-and-window-local`): there is no workspace state here and no
// file written, so closing the editor is the end of it. Revealing one range reveals one range —
// a second secret in the same document stays concealed, because a user who revealed a credential
// to read it has not consented to reveal the rest of them.

/** A position on the wire, in the encoding the client negotiated at `initialize`. */
export interface Position {
  line: number;
  character: number;
}

/** A range on the wire, half-open at `end` as LSP has it. */
export interface Range {
  start: Position;
  end: Position;
}

/**
 * One range the server answered, and why it answered it.
 *
 * `kind` is an open string (ADR 0101 § 1) and today is always `secretLiteral`. It is carried rather
 * than matched on: a later qualifier gives the server a second spelling, and a client that switched
 * on the one it knows would silently draw nothing for the new one.
 */
export interface Redaction {
  range: Range;
  kind: string;
}

/** Where one range is, as a string, so a reveal can name a range across two answers. */
function identity(range: Range): string {
  return `${range.start.line}:${range.start.character}-${range.end.line}:${range.end.character}`;
}

/**
 * Whether `range` spans less of the document than `than`, for choosing between two that both cover
 * a cursor. Lines first: a range over fewer lines is narrower whatever its columns say.
 */
function narrower(range: Range, than: Range): boolean {
  const lines = range.end.line - range.start.line;
  const others = than.end.line - than.start.line;
  return lines !== others
    ? lines < others
    : range.end.character - range.start.character < than.end.character - than.start.character;
}

/** Whether `at` is inside `range`, its closing edge included so a cursor against it still hits. */
function covers(range: Range, at: Position): boolean {
  const after = at.line > range.start.line
    || (at.line === range.start.line && at.character >= range.start.character);
  const before = at.line < range.end.line
    || (at.line === range.end.line && at.character <= range.end.character);
  return after && before;
}

/** The concealment of one window: every document's last answer, keyed by URI. */
export class Concealment {
  private readonly answers = new Map<string, Redaction[]>();
  private readonly reveals = new Map<string, Set<string>>();

  /**
   * Replace what `uri` conceals with the server's latest answer.
   *
   * A reveal survives this only while its range is still one of the ranges answered. An edit that
   * moved the bytes re-conceals them, which is the direction that cannot leave a decoration off a
   * range nobody has looked at.
   */
  hold(uri: string, answered: readonly Redaction[]): void {
    this.answers.set(uri, [...answered]);
    const revealed = this.reveals.get(uri);
    if (revealed !== undefined) {
      const answeredNow = new Set(answered.map((redaction) => identity(redaction.range)));
      for (const range of revealed) {
        if (!answeredNow.has(range)) {
          revealed.delete(range);
        }
      }
    }
  }

  /**
   * Reveal the one range of `uri` that `at` is inside, and answer whether there was one.
   *
   * The innermost, when an answer ever nests one range inside another: the narrower range is the
   * one the user pointed at, and revealing the wider one would uncover bytes they did not ask for.
   */
  reveal(uri: string, at: Position): boolean {
    const under = this.held(uri).filter((redaction) => covers(redaction.range, at));
    if (under.length === 0) {
      return false;
    }
    const narrowest = under.reduce((tightest, redaction) =>
      narrower(redaction.range, tightest.range) ? redaction : tightest);
    const revealed = this.reveals.get(uri) ?? new Set<string>();
    revealed.add(identity(narrowest.range));
    this.reveals.set(uri, revealed);
    return true;
  }

  /** Re-conceal every revealed range in this window, which `nvs.hideSecrets` is. */
  hide(): void {
    this.reveals.clear();
  }

  /** The ranges of `uri` the user has revealed, which are drawn as the plain text they are. */
  revealed(uri: string): Range[] {
    const revealed = this.reveals.get(uri);
    return revealed === undefined
      ? []
      : this.held(uri).map((redaction) => redaction.range).filter((range) => revealed.has(identity(range)));
  }

  /** Whether the server has answered for `uri` at all, which an empty answer still counts as. */
  knows(uri: string): boolean {
    return this.answers.has(uri);
  }

  /** Every range `uri` last answered. */
  held(uri: string): Redaction[] {
    return this.answers.get(uri) ?? [];
  }

  /** The ranges drawn concealed in `uri` right now: everything held, less what is revealed. */
  concealed(uri: string): Range[] {
    const revealed = this.reveals.get(uri);
    return this.held(uri)
      .map((redaction) => redaction.range)
      .filter((range) => revealed === undefined || !revealed.has(identity(range)));
  }

  /**
   * Drop everything held for `uri`.
   *
   * The editor for a document closing is what calls this, and it is deliberately the one thing that
   * empties the map: `rule:security/reveal-is-explicit-and-window-local` keeps the state of a
   * concealment inside the window that drew it, so nothing about it outlives the editor. The
   * reveals go with the answers: reopening the document conceals every range again.
   */
  forget(uri: string): void {
    this.answers.delete(uri);
    this.reveals.delete(uri);
  }
}
