A viewer cannot read a caption and watch the stage when the two are apart: the eye goes to one, and
what happens in the other is missed. So a step's caption is a bubble on the stage, beside the part it
is about, with a tail that points at the place where the step happens. `stage.say(part, { side,
point?, focus?, lift?, hold? })` in `website/src/components/walkthrough/engine/stage.ts` places it,
once per step, at the point in the step where it is to be read.

While the caption shows:

- **The panels it is not about blur**, on top of the camera's dimming, so one thing is sharp. `focus`
  names the parts that stay sharp, and by default it is the part the caption points at.
- **The scene slows almost to a stop** (`Timeline.hold`) for a slow reader's time: 0.7 s and 0.24 s a
  word, less the time the words take to arrive. What a scene authors after `say` happens after the
  reading. Whatever already moves keeps moving, slowly, so the frame never freezes.
- **The bubble keeps its size while the camera zooms.** The stage places it in CSS pixels from the
  camera's frame, and its font follows the stage's width. When the part fills the shot, the shot leaves
  `room` on the bubble's side. `lift` moves the bubble away from something next to its point, with a
  line back to the point.

Every frame stays a function of scene time: the hold is a speed track, like the moment's slow motion,
and the speaker-notes window's clock counts it. A step without `say`, such as a step of the parts
gallery, shows its caption at the bottom of the stage.

`tools/nv/test/walkthrough-briefs.test.ts` holds every scene to one `say` per step. Where the bubble
goes in each layout is a judgement no check makes: the frames from `bun nv walkthrough --frames` are
looked at in both layouts and both themes.

What it spends is time and stage: every step lasts longer in real time, and the bubble covers part of
the stage, so a shot leaves room for it. Nothing of it runs on the request path.

**Declined:**
- **Captions under the stage.** The eye leaves the stage to read and misses what moves meanwhile.
- **Stopping the scene for each caption.** A frozen frame looks like a pause or a fault, and the viewer
  would have to press play again.
- **A caption inside the world, scaled by the camera.** Its text size would change with every shot,
  and on a phone's whole-world shot it would be too small to read.
