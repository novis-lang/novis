/**
 * A walkthrough's brief: what it is for, written before its steps. Every scene exports one as `brief`,
 * and every part, motion and caption of the scene is judged against `moment`. The rule is
 * `website/a-walkthrough-has-one-moment` in docs/rules/, and `tools/nv/test/walkthrough-briefs.test.ts`
 * holds each scene to it.
 */
export interface Brief {
  /** The one thing the viewer understands at the end, in their own words. */
  moment: string
  /** What the viewer expects before the moment, which the moment corrects. */
  expectation: string
  /** What on the stage shows that the moment is true. */
  proof: string
  /** True things the scene leaves out, because they would pull the eye away from the moment. */
  leavesOut: string[]
  /** Where the viewer goes next. */
  then: string
}
