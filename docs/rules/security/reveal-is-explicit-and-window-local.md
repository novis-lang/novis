A reveal is per range, window-local, and dropped when the editor for that document closes. It is not
written to workspace state and does not survive a reload.

The threat model is an unattended screen, so a reveal that outlives the moment it was needed is the
same as no redaction at all — and a user who revealed one credential to read it has not consented to
reveal every credential in the workspace for the rest of the week.

There is **no automatic reveal and no automatic re-conceal on a signal**, because there is no signal:
nothing reports that a window is being shared, recorded or projected. Anything that looked like one
would be a guess with a security failure attached, so redaction is unconditional by default and the
user is the only thing that turns it off.

**Not on disk.** There is no language server in the tree.
