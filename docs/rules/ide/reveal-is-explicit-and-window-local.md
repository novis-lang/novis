A reveal is per range, window-local, and dropped when the editor for that document closes. It is not
written to workspace state and does not survive a reload.

The threat model is an unattended screen, so a reveal that outlives the moment it was needed is the
same as no redaction at all — and a user who revealed one credential to read it has not consented to
reveal every credential in the workspace for the rest of the week.

There are three ways to ask, and they divide on what they leave behind rather than on how they are
spelled. The hover's command link and the `nvs.revealSecret` command are **held**: the range stays
uncovered until the editor closes or `nvs.hideSecrets` runs, so each one is a decision the user made
about one range and can be pointed at afterwards. **A cursor inside a range uncovers it and holds
nothing** — the range is covered again the instant the cursor leaves, and nothing anywhere records
that it was ever uncovered.

The cursor is admitted because it cannot leave a screen uncovered behind the user. A held reveal
can: it survives every scroll, edit and tab switch until the window is told otherwise, which is
exactly the exposure the paragraph above is about. A reveal that is a function of where the caret is
right now expires on the next arrow key, so the worst it can do is show the value the user is
already looking at. That is a smaller surface than the two held reveals it sits beside, not a larger
one — the reason to write it down is that it *reads* like the opposite.

There is still **no automatic re-conceal on a signal**, because there is no signal: nothing reports
that a window is being shared, recorded or projected. Anything that looked like one would be a guess
with a security failure attached, so redaction is unconditional by default and the user is the only
thing that turns it off.
