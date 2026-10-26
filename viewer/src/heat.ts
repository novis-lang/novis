// The step of the coverage heatmap a line is drawn at (`views/coverage.tsx`). It imports nothing, so the
// unit tests read it without a browser.

/** The steps of the scale. `style.css` has a `heat-<n>` class for each. */
export const STEPS = 4;

/** The step of a line that ran `count` times, in a file whose most-run line ran `most` times: `0` for a
 *  line that did not run, and `1` to `STEPS` on a log scale, so one hot loop does not flatten the rest. */
export function heat(count: number, most: number): number {
  if (count <= 0 || most <= 0) return 0;
  return Math.max(1, Math.ceil((STEPS * Math.log1p(count)) / Math.log1p(most)));
}
