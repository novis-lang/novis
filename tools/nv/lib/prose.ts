// Reading Markdown prose. This is the one place anything looks inside a prose file: its headings, a
// section under a heading, and the citations and links in it. Every reader here skips fenced code
// blocks, so a `#` comment or a `rule:` token in an example is never mistaken for the real thing.

export interface Heading {
  level: number;
  text: string;
  /** 1-based. */
  line: number;
}

export type CitationKind = "rule" | "decision" | "link";

export interface Citation {
  kind: CitationKind;
  /** A rule id (`types/conversion`), a four-digit decision number, or a link's target as written. */
  target: string;
  /** 1-based. */
  line: number;
}

const FENCE_OPEN = /^ {0,3}(`{3,}|~{3,})/;
const HEADING = /^ {0,3}(#{1,6})(?:[ \t]+(.*?))?(?:[ \t]+#+)?[ \t]*$/;
const RULE = /rule:([a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9-]*)/g;
const DECISION = /(?:\bADRs? (\d{4})\b|decisions\/(\d{4})\.md|(?<![\w.])(\d{4}) §)/g;
const LINK = /\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;
const CODE_SPAN = /(`+)[\s\S]*?\1/g;

function lines(text: string): string[] {
  return text.replace(/\r\n?/g, "\n").split("\n");
}

/** For each line, whether it is part of a fenced code block, fence lines included. */
export function fenced(all: string[]): boolean[] {
  const out: boolean[] = [];
  let open: string | null = null;
  for (const line of all) {
    if (open === null) {
      const m = FENCE_OPEN.exec(line);
      if (m && m[1]) {
        // A backtick fence's info string may not itself hold a backtick.
        const isFence = m[1][0] !== "`" || !line.slice(line.indexOf(m[1]) + m[1].length).includes("`");
        if (isFence) {
          open = m[1];
          out.push(true);
          continue;
        }
      }
      out.push(false);
    } else {
      const close = new RegExp(`^ {0,3}${open[0] === "`" ? "`" : "~"}{${open.length},}[ \\t]*$`);
      if (close.test(line)) open = null;
      out.push(true);
    }
  }
  return out;
}

/** Every ATX heading outside a fence, in order. */
export function headings(text: string): Heading[] {
  const all = lines(text);
  const inFence = fenced(all);
  const out: Heading[] = [];
  all.forEach((line, i) => {
    if (inFence[i]) return;
    const m = HEADING.exec(line);
    if (m && m[1]) out.push({ level: m[1].length, text: (m[2] ?? "").trim(), line: i + 1 });
  });
  return out;
}

/**
 * The section a heading opens: the heading line through the line before the next heading of the same
 * or a higher level. `select` is either the heading's text, matched exactly and then as a prefix, or a
 * written-out heading such as `## 4`, which also fixes the level. Null when no heading matches.
 */
export function section(text: string, select: string): string | null {
  const all = lines(text);
  const hs = headings(text);
  const written = /^(#{1,6})\s+(.*)$/.exec(select.trim());
  const level = written?.[1]?.length;
  const want = (written ? written[2] ?? "" : select).trim();
  const candidates = hs.filter((h) => level === undefined || h.level === level);
  const found = candidates.find((h) => h.text === want) ?? candidates.find((h) => h.text.startsWith(want));
  if (!found) return null;
  const next = hs.find((h) => h.line > found.line && h.level <= found.level);
  const end = next ? next.line - 1 : all.length;
  return all.slice(found.line - 1, end).join("\n").replace(/\n+$/, "") + "\n";
}

/**
 * Every citation and link outside a fence: `rule:<topic>/<slug>`, a decision named as `ADR NNNN`,
 * `decisions/NNNN.md` or `NNNN §`, and a Markdown link's target. A rule or a decision is read inside
 * an inline code span too, because that is how this tree writes them; a link inside a code span is
 * an example of a link, and is skipped. One citation per line per kind and target.
 */
export function citations(text: string): Citation[] {
  const all = lines(text);
  const inFence = fenced(all);
  const out: Citation[] = [];
  all.forEach((line, i) => {
    if (inFence[i]) return;
    const seen = new Set<string>();
    const add = (kind: CitationKind, target: string) => {
      const key = `${kind}\0${target}`;
      if (seen.has(key)) return;
      seen.add(key);
      out.push({ kind, target, line: i + 1 });
    };
    for (const m of line.matchAll(RULE)) if (m[1]) add("rule", m[1]);
    for (const m of line.matchAll(DECISION)) {
      const n = m[1] ?? m[2] ?? m[3];
      if (n) add("decision", n);
    }
    for (const m of line.replace(CODE_SPAN, (span) => " ".repeat(span.length)).matchAll(LINK)) {
      if (m[1]) add("link", m[1]);
    }
  });
  return out;
}
