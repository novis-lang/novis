// The website's rulebook: `website/src/data/rules.json`, which the components and the sidebar read,
// and one page per chapter and per section under `website/src/content/docs/docs/rules/`. The rules
// and their order come from `data/rules/`, the prose from `docs/rules/<topic>/<slug>.md`, and where
// a chapter is cut into pages from `website/config/rule-sections.mjs`. Every file under the pages
// directory is this renderer's except the handwritten hub, `index.mdx`.
//
// A page is plain `.md`, not `.mdx`: rule prose is full of `#[Attribute(…)]`, `{field: T}` and `<T>`,
// which MDX reads as JSX. The per-rule chrome is raw HTML around the prose, and a Markdown HTML block
// ends at a blank line, so the prose between the wrappers is parsed as ordinary Markdown.
//
// Every rule carries `<a id="<slug>">` of its own, so a link to it stays stable whatever its title
// becomes, and every `rule:` citation in the prose is rewritten to point at it.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { MARKER, type Output, type Renderer } from "../lib/render.ts";
import { load } from "../lib/store.ts";
import { rule as ruleRecord } from "../schema/rule.ts";
import { topic as topicRecord } from "../schema/topic.ts";

export const PAGES_DIR = "website/src/content/docs/docs/rules";
export const DATA_FILE = "website/src/data/rules.json";
/** The one handwritten page under `PAGES_DIR`. */
export const HANDWRITTEN = ["index.mdx"];

/** The route the whole rulebook hangs under. */
const ROOT_URL = "/docs/rules";

interface Section {
  slug: string;
  title: string;
  blurb: string;
  from: string;
}

interface SiteConfig {
  githubFile(path: string): string;
  decisionRecord(number: string): string;
  ruleSections: Record<string, Section[]>;
  chapterLeads: Record<string, string>;
}

interface Rule {
  id: string;
  slug: string;
  title: string;
  status: string;
  because: string[];
  diverges: string | null;
  seeAlso: string[];
  guardedBy: string[];
  body: string;
  url: string;
  href: string;
}

interface CutSection extends Section {
  url: string;
  rules: Rule[];
}

interface Chapter {
  topic: string;
  title: string;
  url: string;
  rules: Rule[];
  sections: CutSection[];
  single: boolean;
  lead: string;
}

async function siteConfig(root: string): Promise<SiteConfig> {
  const site = await import(join(root, "website", "config", "site.mjs"));
  const cut = await import(join(root, "website", "config", "rule-sections.mjs"));
  return { githubFile: site.githubFile, decisionRecord: site.decisionRecord, ruleSections: cut.ruleSections, chapterLeads: cut.chapterLeads };
}

/** Every chapter with its rules, in the rulebook's own order. */
function chapters(root: string): Chapter[] {
  const bad = [...load(topicRecord, root), ...load(ruleRecord, root)].filter((r) => r.issues.length > 0);
  if (bad.length > 0) throw new Error(`${bad.map((r) => r.path).join(", ")} fail their schema; \`bun nv check\` names why`);
  const rules = new Map(load(ruleRecord, root).map((r) => [r.id, r.value]));
  return load(topicRecord, root)
    .sort((a, b) => a.value.order - b.value.order || a.id.localeCompare(b.id))
    .map((t) => ({
      topic: t.id,
      title: t.value.title,
      url: `${ROOT_URL}/${t.id}/`,
      sections: [],
      single: false,
      lead: "",
      rules: t.value.rules.map((id) => {
        const r = rules.get(id);
        if (!r) throw new Error(`data/rules/${t.id}.json lists ${id}, which has no record`);
        const slug = id.slice(id.indexOf("/") + 1);
        const bodyFile = join(root, "docs", "rules", t.id, `${slug}.md`);
        return {
          id,
          slug,
          title: r.title,
          status: r.status,
          because: r.because,
          diverges: r.divergesFromPhp ?? null,
          seeAlso: r.seeAlso,
          guardedBy: r.guardedBy,
          body: existsSync(bodyFile) ? readFileSync(bodyFile, "utf8").replace(/\r\n?/g, "\n").replace(/\s+$/, "") : "",
          url: "",
          href: "",
        };
      }),
    }));
}

/**
 * One chapter cut into its configured sections. The cut is by first-rule slug rather than by index,
 * so a `from` that names no rule is a stale cut and fails the render rather than merging two sections.
 */
function sectionsFor(chapter: Chapter, cut: Record<string, Section[]>): CutSection[] {
  const configured = cut[chapter.topic];
  if (!configured || configured.length === 0) throw new Error(`chapter "${chapter.topic}" has no entry in website/config/rule-sections.mjs`);
  if (configured[0]!.from !== chapter.rules[0]?.slug) {
    throw new Error(
      `chapter "${chapter.topic}": the first section must start at the chapter's first rule ` +
        `("${chapter.rules[0]?.slug}"), and it starts at "${configured[0]!.from}"`,
    );
  }
  const starts = new Map(configured.map((s) => [s.from, s]));
  const single = configured.length === 1;
  const sections: CutSection[] = [];
  for (const rule of chapter.rules) {
    const opening = starts.get(rule.slug);
    if (opening) {
      starts.delete(rule.slug);
      // A chapter cut into one section has no page of its own: its rules are on the chapter page.
      sections.push({ ...opening, url: single ? chapter.url : `${chapter.url}${opening.slug}/`, rules: [] });
    }
    sections[sections.length - 1]!.rules.push(rule);
  }
  if (starts.size > 0) {
    throw new Error(
      `chapter "${chapter.topic}": section${starts.size === 1 ? "" : "s"} cut at ` +
        `${[...starts.keys()].map((s) => `"${s}"`).join(", ")}, which no rule in the chapter is named`,
    );
  }
  return sections;
}

const escapeHtml = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** The sliver of inline Markdown the structured fields use: code spans and bold. */
const inlineHtml = (s: string) =>
  escapeHtml(s)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");

/** A rule title as a link tooltip: plain text, no markup, no quotes to close early. */
const plainTitle = (s: string) => s.replace(/[`*"]/g, "");

/**
 * `divergesFromPhp` is written as a clause, because the rulebook renders it after a comma. Here it
 * stands under a heading of its own, so it starts a sentence, unless it starts with code.
 */
const asSentence = (s: string) => (/^[a-z]/.test(s) ? s[0]!.toUpperCase() + s.slice(1) : s);

const STATUS_LABEL: Record<string, string> = { shipped: "Shipped", designed: "Designed" };
const statusLabel = (s: string) => STATUS_LABEL[s] ?? s;

function counts(rules: Rule[]) {
  const shipped = rules.filter((r) => r.status === "shipped").length;
  const diverging = rules.filter((r) => r.diverges).length;
  return { total: rules.length, shipped, designed: rules.length - shipped, diverging };
}

function renderCounts(rules: Rule[]): string {
  const c = counts(rules);
  const cell = (value: number, label: string, kind: string) =>
    `<div class="nv-count" data-kind="${kind}"><span class="nv-count-value">${value}</span>` +
    `<span class="nv-count-label">${label}</span></div>`;
  return (
    '<div class="nv-counts">' +
    cell(c.total, c.total === 1 ? "rule" : "rules", "total") +
    cell(c.shipped, "shipped", "shipped") +
    cell(c.designed, "designed", "designed") +
    cell(c.diverging, c.diverging === 1 ? "differs from PHP" : "differ from PHP", "php") +
    "</div>"
  );
}

/** The compact list of what is on a page, above the prose. */
function renderRuleList(rules: Rule[], linked: boolean): string {
  const items = rules
    .map(
      (rule) =>
        `<li><a href="${linked ? rule.href : `#${rule.slug}`}">${inlineHtml(rule.title)}</a>` +
        `<span class="nv-rule-list-status" data-status="${rule.status}">${statusLabel(rule.status)}</span>` +
        (rule.diverges ? '<span class="nv-rule-list-flag" title="Differs from PHP">PHP</span>' : "") +
        "</li>",
    )
    .join("");
  return `<ol class="nv-rule-list">${items}</ol>`;
}

interface Link {
  link: string;
  label: string;
}

/**
 * A page's front matter. It carries the render marker, and the `GENERATED FILE` line `bun nv links`
 * skips a page by: its links are site routes, which resolve in Astro's router and never on disk.
 */
function frontmatter(title: string, description: string, prev: Link | undefined, next: Link | undefined): string {
  const lines = [
    "---",
    `# ${MARKER}`,
    "# GENERATED FILE from data/rules/ and docs/rules/: its links are site routes.",
    `title: ${JSON.stringify(title)}`,
    `description: ${JSON.stringify(description)}`,
    "editUrl: false",
    "lastUpdated: false",
    // The page opens with its own contents list; Starlight's would repeat it as wrapped sentences.
    "tableOfContents: false",
    prev ? `prev:\n  link: ${prev.link}\n  label: ${JSON.stringify(prev.label)}` : "prev: false",
    next ? `next:\n  link: ${next.link}\n  label: ${JSON.stringify(next.label)}` : "next: false",
    "---",
    "",
  ];
  return lines.join("\n");
}

/** Markdown as the plain sentence a `description` meta tag carries. */
function summarize(text: string, limit = 155): string {
  const flat = text
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/\s+/g, " ")
    .trim();
  if (flat.length <= limit) return flat;
  return `${flat.slice(0, flat.lastIndexOf(" ", limit - 1))}…`;
}

export async function renderWebsiteRules(root: string): Promise<Output[]> {
  const site = await siteConfig(root);
  const topics = chapters(root);
  for (const chapter of topics) {
    chapter.sections = sectionsFor(chapter, site.ruleSections);
    chapter.single = chapter.sections.length === 1;
    // A one-section chapter leads with that section's blurb; every other chapter owes a lead.
    const lead = chapter.single ? chapter.sections[0]!.blurb : site.chapterLeads[chapter.topic];
    if (!lead) throw new Error(`chapter "${chapter.topic}" has no entry in chapterLeads`);
    chapter.lead = lead;
    for (const section of chapter.sections) {
      for (const rule of section.rules) {
        rule.url = section.url;
        rule.href = `${section.url}#${rule.slug}`;
      }
    }
  }
  const byId = new Map<string, Rule>();
  for (const chapter of topics) for (const rule of chapter.rules) byId.set(rule.id, rule);

  const dangling: string[] = [];
  /**
   * The `rule:<topic>/<slug>` citations in a body as links, whether bare or in a code span. Fenced
   * code is left as written.
   */
  const linkCitations = (body: string, from: string) => {
    const citation = /`rule:([a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9-]*)`|rule:([a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9-]*)/g;
    return body
      .split(/(^```[\s\S]*?^```)/m)
      .map((chunk, i) =>
        i % 2 === 1
          ? chunk
          : chunk.replace(citation, (_whole, quoted: string | undefined, bare: string | undefined) => {
              const id = (quoted ?? bare)!;
              const target = byId.get(id);
              if (!target) {
                dangling.push(`${from} cites rule:${id}, which no chapter declares`);
                return `\`${id}\``;
              }
              return `[\`${id}\`](${target.href} "${plainTitle(target.title)}")`;
            }),
      )
      .join("");
  };

  const renderRule = (rule: Rule): string => {
    const out = [`<div class="nv-rule" id="${rule.slug}">`, "", `## ${rule.title}`, "", '<div class="nv-rule-tags">'];
    out.push(`<span class="nv-rule-status" data-status="${rule.status}">${statusLabel(rule.status)}</span>`);
    if (rule.diverges) out.push('<span class="nv-rule-flag">Differs from PHP</span>');
    out.push(`<a class="nv-rule-id" href="#${rule.slug}"><code>${escapeHtml(rule.id)}</code></a>`, "</div>", "");
    if (rule.body) out.push(linkCitations(rule.body, rule.id), "");
    if (rule.diverges) {
      out.push(
        '<aside class="nv-rule-diverges">',
        '<p class="nv-rule-diverges-label">Where this differs from PHP</p>',
        `<p>${inlineHtml(asSentence(rule.diverges))}</p>`,
        "</aside>",
        "",
      );
    }
    const meta: string[] = [];
    const row = (dt: string, dd: string[]) => `<div class="nv-rule-meta-row"><dt>${dt}</dt><dd>${dd.join(" ")}</dd></div>`;
    const seeAlso = rule.seeAlso.map((id) => byId.get(id)).filter((r): r is Rule => r !== undefined);
    if (seeAlso.length > 0) {
      meta.push(row("See also", seeAlso.map((r) => `<a href="${r.href}" title="${escapeHtml(plainTitle(r.title))}"><code>${r.id}</code></a>`)));
    }
    if (rule.because.length > 0) meta.push(row("Decided in", rule.because.map((n) => `<a href="${site.decisionRecord(n)}">record ${n}</a>`)));
    if (rule.guardedBy.length > 0) {
      meta.push(row("Guarded by", rule.guardedBy.map((p) => `<a href="${site.githubFile(p)}"><code>${escapeHtml(p)}</code></a>`)));
    }
    if (meta.length > 0) out.push(`<dl class="nv-rule-meta">${meta.join("")}</dl>`, "");
    out.push("</div>");
    return out.join("\n");
  };

  // The page before and after each one, in reading order across the whole rulebook.
  const reading: Link[] = [];
  for (const chapter of topics) {
    reading.push({ link: chapter.url, label: chapter.title });
    if (!chapter.single) for (const s of chapter.sections) reading.push({ link: s.url, label: s.title });
  }
  const neighbours = (link: string) => {
    const i = reading.findIndex((page) => page.link === link);
    return { prev: reading[i - 1], next: reading[i + 1] };
  };

  const outputs: Output[] = [];
  const page = (sub: string, lines: string[]) => outputs.push({ path: `${PAGES_DIR}/${sub}`, text: lines.join("\n") });

  for (const chapter of topics) {
    const nav = neighbours(chapter.url);
    const body = [
      frontmatter(chapter.title, summarize(chapter.lead), nav.prev, nav.next),
      `<p class="nv-section-lead">${inlineHtml(chapter.lead)}</p>`,
      "",
      renderCounts(chapter.rules),
      "",
    ];
    if (chapter.single) {
      body.push(renderRuleList(chapter.rules, false), "");
      for (const rule of chapter.rules) body.push(renderRule(rule), "");
    } else {
      body.push('<div class="nv-sections">');
      for (const section of chapter.sections) {
        body.push(
          '<section class="nv-section">',
          `<h2 class="nv-section-title"><a href="${section.url}">${inlineHtml(section.title)}</a>` +
            `<span class="nv-section-count">${section.rules.length}</span></h2>`,
          `<p class="nv-section-blurb">${inlineHtml(section.blurb)}</p>`,
          renderRuleList(section.rules, true),
          "</section>",
        );
      }
      body.push("</div>");
    }
    page(`${chapter.topic}/index.md`, body);

    if (chapter.single) continue;
    for (const section of chapter.sections) {
      const sNav = neighbours(section.url);
      const lines = [
        frontmatter(section.title, summarize(section.blurb), sNav.prev, sNav.next),
        `<p class="nv-section-lead">${inlineHtml(section.blurb)}</p>`,
        "",
        renderCounts(section.rules),
        "",
        renderRuleList(section.rules, false),
        "",
      ];
      for (const rule of section.rules) lines.push(renderRule(rule), "");
      page(`${chapter.topic}/${section.slug}.md`, lines);
    }
  }

  if (dangling.length > 0) throw new Error(`${dangling.length} dangling citation(s):\n  ${dangling.join("\n  ")}`);

  const data = {
    totals: counts(topics.flatMap((t) => t.rules)),
    topics: topics.map((chapter) => ({
      topic: chapter.topic,
      title: chapter.title,
      url: chapter.url,
      single: chapter.single,
      counts: counts(chapter.rules),
      sections: chapter.sections.map((section) => ({
        slug: section.slug,
        title: section.title,
        blurb: section.blurb,
        url: section.url,
        counts: counts(section.rules),
        rules: section.rules.map((rule) => ({
          id: rule.id,
          slug: rule.slug,
          title: rule.title,
          status: rule.status,
          href: rule.href,
          diverges: rule.diverges,
        })),
      })),
    })),
  };
  outputs.push({ path: DATA_FILE, text: `${JSON.stringify(data, null, 2)}\n` });
  return outputs;
}

export const websiteRules: Renderer = {
  name: "website-rules",
  render: renderWebsiteRules,
  owns: { dir: PAGES_DIR, keep: HANDWRITTEN },
};
