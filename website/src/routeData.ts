/**
 * Starlight route data middleware (registered via `routeMiddleware` in
 * astro.config.ts): prefix the site's `base` path onto hero action links.
 * They are authored root-absolute in frontmatter, which no markdown pipeline
 * or component of ours ever sees — Starlight's Hero renders them verbatim.
 */
import { defineRouteMiddleware } from '@astrojs/starlight/route-data'
import { withBase } from '~/lib/base'

export const onRequest = defineRouteMiddleware((context) => {
  const { hero } = context.locals.starlightRoute.entry.data
  for (const action of hero?.actions ?? []) {
    if (action.link?.startsWith('/') && !action.link.startsWith('//')) {
      action.link = withBase(action.link)
    }
  }
})
