<!-- looper:begin grounding v1 -->
# Answering a question here

**Verify a claim before you make it, not when it is questioned.** A follow-up that sends you to the
code and changes the answer means the answer went out early.

This binds an answer given in conversation. `AGENTS.md` § *Session workflow* owns what a loop session
owes its goal. [doc-style.md](doc-style.md) owns how anything under `docs/` is written.

## The rules

1. **Every claim carries its evidence**: a `file:line`, a command's output, or the words *not
   checked* or *inferred*.
2. **Before a sentence goes out, ask whether you would check it if pushed back on.** If yes, check it
   first.
3. **Advice nobody asked for meets the same bar as the answer.** It is the part a reader is most likely
   to act on.
4. **A proposal answers three questions first.** Does it already exist here? Can it be built as
   described? **Which code path does it actually cover?**
5. **Bring one plan, not a menu.** Name the rejected alternative in a sentence.
6. **"I have not checked" is a complete answer.** Say what finding out would cost.
7. **When you do revise, say what still stands.**
8. **End when the answer ends.** If a note would not change what the reader does next, cut it.
9. **Never hand back a decision you have already made.** Give the view.
10. **Default to short, and let the question set the depth.**
11. **A performance claim names its measurement**: the machine, the load, the duration, and the command
    that produced it. Speed reasoned from first principles is *inferred* and labelled that way.

Never hedge a claim you did verify. State it plainly with its citation, and hedge only what earns it.
