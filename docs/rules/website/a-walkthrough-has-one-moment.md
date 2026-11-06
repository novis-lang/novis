A walkthrough is an interactive, video-like explainer on the website: parts on a stage, a camera, and a
caption for each step, played or stepped through. Each one exists for **one moment**, the step where
what the viewer expected turns out wrong and the feature's point lands.

The scene is written from its **brief**, which its module exports as `brief` (`Brief` in
`website/src/components/walkthrough/engine/brief.ts`) before any step is written:

- `moment`: the one thing the viewer understands at the end, in their words.
- `expectation`: what they expect before it, which the moment corrects.
- `proof`: what on the stage shows the moment is true.
- `leavesOut`: true things the scene leaves out, because they would pull the eye away from the moment.
- `then`: where the viewer goes next. The text above the stage on the walkthrough's page links it,
  so a viewer who wants more follows the link from where they started.

Exactly one step is written with `moment: true`. The player marks it gold on the scrub bar, its
caption is gold, and the scene slows down for it.

**Everything on the stage serves the moment.** A part, a motion or a caption that is no step toward it
is cut, however true. What is cut is named in `leavesOut`, so the next author knows it was left out on
purpose: the `afterResponse` walkthrough shows no memory or CPU meter, no limit and no failure case. A
second idea is a second walkthrough, and the whole feature is the page `then` names.

A viewer remembers one thing from something they watch. Two moments compete for that one place, and the
stillness and slow motion that make a moment land cannot be spent twice. A brief written first is what
the stage is judged against, and naming what is left out stops the next author from adding it back.

`tools/nv/test/walkthrough-briefs.test.ts` holds every scene under
`website/src/components/walkthrough/scenes/` to the shape: a `brief` with every field filled and exactly
one `moment: true`, and holds the page that shows each scene to at least one link in its text. Whether
the stage serves the moment is a judgement no check makes, and the brief is
where it is written down. The parts gallery, `in-depth/walkthrough-parts`, shows parts and is not a
walkthrough, so it has no brief.

What it spends is coverage: a walkthrough says less than a reference page, and a feature with two
surprises needs two walkthroughs. Nothing of it runs on the request path.

**Declined:**
- **A walkthrough that tours the whole feature.** It becomes a slower reference page, and the reference
  page exists.
- **Memory and CPU meters in the `afterResponse` walkthrough.** They are true, and they pull the eye away
  from the request that is still running.
- **A moment on every step.** When every step is marked, none of them is.
