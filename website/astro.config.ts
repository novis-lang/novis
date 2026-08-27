import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import sitemap from '@astrojs/sitemap';
import starlight from '@astrojs/starlight';
import starlightLinksValidator from 'starlight-links-validator';

import { SITE_BASE, SITE_URL, site } from './src/site.config';

/**
 * The Novis TextMate grammar is generated from the compiler's keyword table by
 * `python site.py sync` (see `tools/gen_grammar.py`). It is deliberately not
 * committed: the keyword list has exactly one home, `crates/nvs-syntax`, and a
 * committed copy here would be a second one.
 *
 * If it is missing we fall back to no custom language rather than failing the
 * build, so that someone who cloned only `website/` still gets a site — code
 * blocks just render unhighlighted, and `site.py check` says why.
 */
const grammarUrl = new URL('./src/grammars/nvs.tmLanguage.json', import.meta.url);
const nvsGrammar = existsSync(fileURLToPath(grammarUrl))
  ? JSON.parse(readFileSync(grammarUrl, 'utf8'))
  : null;

// The pre-alpha notice is a component override rather than a config value:
// Starlight's own `banner` is per-page frontmatter, and this fact is not
// per-page. See src/components/overrides/Banner.astro.

export default defineConfig({
  site: SITE_URL,
  base: SITE_BASE,
  trailingSlash: 'always',
  build: { format: 'directory' },

  // No telemetry, no remote fonts, no CDN. Every byte the browser fetches is
  // built from this tree, which is what makes an offline build possible.
  integrations: [
    starlight({
      title: site.name,
      description: site.tagline,
      tagline: site.tagline,

      // Light is the default. Starlight ships dark-first and follows the OS;
      // our ThemeProvider override flips the fallback. The toggle is untouched.
      components: {
        ThemeProvider: './src/components/overrides/ThemeProvider.astro',
        SiteTitle: './src/components/overrides/SiteTitle.astro',
        Banner: './src/components/overrides/Banner.astro',
      },

      customCss: ['./src/styles/tokens.css', './src/styles/site.css'],

      social: [{ icon: 'github', label: 'Source', href: site.repo }],

      editLink: {
        baseUrl: `${site.repo}/edit/${site.repoRef}/website/`,
      },

      lastUpdated: true,

      pagination: true,

      expressiveCode: {
        // One theme per colour scheme, both shipped with Starlight, both
        // meeting AA against our own surfaces.
        themes: ['github-light', 'github-dark'],
        shiki: {
          langs: nvsGrammar ? [nvsGrammar] : [],
        },
        styleOverrides: {
          borderRadius: '4px',
          borderColor: 'var(--nv-rule)',
          codeFontSize: '0.875rem',
          codeLineHeight: '1.65',
          frames: {
            shadowColor: 'transparent',
          },
        },
      },

      sidebar: [
        {
          label: 'Why Novis',
          items: [
            { label: 'Who it is for', link: '/why/' },
            { label: 'What it refuses to do', link: '/why/non-goals/' },
            { label: 'Compared to PHP and Python', link: '/why/comparisons/' },
          ],
        },
        {
          label: 'Get started',
          items: [
            { label: 'Install', link: '/docs/start/install/' },
            { label: 'Your first script', link: '/docs/start/first-script/' },
          ],
        },
        {
          label: 'Language',
          items: [
            { label: 'Types', link: '/docs/language/types/' },
            { label: 'Iteration and generators', link: '/docs/language/iteration/' },
          ],
        },
        {
          label: 'Isolation and safety',
          items: [
            { label: 'Isolated scripts', link: '/docs/safety/isolated-scripts/' },
            { label: 'Secret values', link: '/docs/safety/secret/' },
            { label: 'Taint tracking', link: '/docs/safety/taint/' },
          ],
        },
        {
          label: 'Project',
          items: [
            { label: 'Status and roadmap', link: '/status/' },
            { label: 'Design decisions', link: '/design/' },
          ],
        },
      ],

      plugins: [
        // A broken internal link fails the build rather than shipping. Runs on
        // the generated ADR mirror too, which is where link rot would otherwise
        // appear first.
        starlightLinksValidator({
          errorOnRelativeLinks: false,
          errorOnLocalLinks: false,
        }),
      ],

      head: [
        {
          tag: 'meta',
          attrs: { name: 'theme-color', content: '#ffffff', media: '(prefers-color-scheme: light)' },
        },
        {
          tag: 'meta',
          attrs: { name: 'theme-color', content: '#0f1115', media: '(prefers-color-scheme: dark)' },
        },
      ],
    }),
    mdx(),
    sitemap(),
  ],
});
