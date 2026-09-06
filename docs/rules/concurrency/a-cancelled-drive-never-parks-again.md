A cancellation ends the drive. The drive adds nothing to the cancellation model and inherits it
whole, and both of the scheduler's two answers stop the loop.

The ordinary one is a teardown, and the drive never returns from it. A parked coroutine whose stack
may be unwound is dropped where it stands, and the unwind drops the future with it — closing the
connection through the host language's own drops, and running no Novis frame.

The other is a resume. A stack standing on a helper frame cannot be unwound, so the scheduler tells
it once, on the way back in; there the loop stops and the drive answers nothing. Parking again would
wait on a wake that is not coming, which is a wedged coroutine rather than a slow one, and going on
would serve a request the client is no longer attached to.
