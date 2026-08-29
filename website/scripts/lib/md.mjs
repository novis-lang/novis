/**
 * Minimal markdown → HTML for the spec excerpts the sync scripts embed in
 * generated data (class summaries, surface descriptions). Handles exactly
 * what those excerpts use: paragraphs, bullet lists, inline code, bold,
 * italics and links. Anything fancier stays in real pages.
 */

import path from 'node:path'
import { githubFile } from '../../config/site.mjs'

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

/**
 * Rewrite a spec-relative link target for the site: ADR links become site
 * pages, everything else repo-relative goes to GitHub.
 * @param {string} target link target as written in docs/spec/…
 * @param {string} fromRepoDir repo-relative dir of the source file, e.g. 'docs/spec'
 */
export function rewriteTarget(target, fromRepoDir) {
  const adr = /(?:^|\/)(\d{4})-[^/]*\.md(#.*)?$/.exec(target)
  if (adr && target.includes('adr')) return `/docs/adr/${adr[1]}/${adr[2] ?? ''}`
  if (/^[a-z]+:/i.test(target) || target.startsWith('/')) return target
  const repoPath = path.posix.normalize(path.posix.join(fromRepoDir, target)).split('#')[0]
  if (repoPath.startsWith('..')) return target
  return githubFile(repoPath)
}

/** Inline markdown to HTML. */
export function inlineHtml(md, fromRepoDir) {
  return escapeHtml(md)
    .replace(/\[([^\]]*)\]\(([^)]+)\)/g, (_, text, target) => {
      const href = rewriteTarget(target, fromRepoDir)
      const adr = /^\/docs\/adr\/(\d{4})\//.exec(href)
      return `<a href="${href}"${adr ? ` data-adr="${adr[1]}"` : ''}>${text}</a>`
    })
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\*([^*]+)\*/g, '<em>$1</em>')
}

/** Block-level markdown (paragraphs + bullet lists) to HTML. */
export function blocksHtml(md, fromRepoDir) {
  const out = []
  let paragraph = []
  let list = null

  const flushParagraph = () => {
    if (paragraph.length > 0) {
      out.push(`<p>${inlineHtml(paragraph.join(' '), fromRepoDir)}</p>`)
      paragraph = []
    }
  }
  const flushList = () => {
    if (list) {
      out.push(`<ul>${list.map((item) => `<li>${inlineHtml(item, fromRepoDir)}</li>`).join('')}</ul>`)
      list = null
    }
  }

  for (const line of md.split(/\r?\n/)) {
    const trimmed = line.trim()
    if (trimmed === '') {
      flushParagraph()
      flushList()
    } else if (/^-\s+/.test(trimmed)) {
      flushParagraph()
      if (!list) list = []
      list.push(trimmed.replace(/^-\s+/, ''))
    } else if (list && /^\s+\S/.test(line)) {
      list[list.length - 1] += ' ' + trimmed
    } else {
      flushList()
      paragraph.push(trimmed)
    }
  }
  flushParagraph()
  flushList()
  return out.join('\n')
}
