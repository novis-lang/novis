// The Expressive Code options that are code. The rest are in `astro.config.ts`, and the
// two are merged. A plugin lives here because the `<Code>` component reads the options in
// the Astro config as JSON, and a function is not JSON.
import { taintedRanges } from './config/tainted-ranges.mjs'

export default {
  plugins: [taintedRanges()],
}
