/**
 * Starlight route data middleware (registered via `routeMiddleware` in
 * astro.config.ts). It does two things to every page:
 *
 * - It narrows the sidebar to the area the page is in. The config's sidebar
 *   has one top-level group per area (AREAS and INSTALL in config/site.mjs),
 *   and a page shows only the entries of its own area's group. A page outside
 *   every area, such as the home page or the Impressum, has no sidebar.
 * - It prefixes the site's `base` path onto hero action links. They are
 *   authored root-absolute in frontmatter, which no markdown pipeline or
 *   component of ours ever sees — Starlight's Hero renders them verbatim.
 */
import { defineRouteMiddleware } from '@astrojs/starlight/route-data'
import { AREAS, INSTALL } from '@config/site.mjs'
import { withBase } from '~/lib/base'

const SECTIONS = [...AREAS, INSTALL]

export const onRequest = defineRouteMiddleware((context) => {
  const route = context.locals.starlightRoute

  const section = SECTIONS.find((s) => context.url.pathname.startsWith(withBase(s.href)))
  const group = section && route.sidebar.find((e) => e.type === 'group' && e.label === section.label)
  if (group?.type === 'group') {
    route.sidebar = group.entries
  } else {
    route.sidebar = []
    route.hasSidebar = false
  }

  for (const action of route.entry.data.hero?.actions ?? []) {
    if (action.link?.startsWith('/') && !action.link.startsWith('//')) {
      action.link = withBase(action.link)
    }
  }
})
