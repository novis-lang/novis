# Answering a question here

**Verify a claim before you make it, not when it is questioned.** A follow-up that sends you to the
code and changes the answer means the answer went out early.

The check is cheap, which is the whole argument: `bun nv brief --where <keyword>` names the
file that owns a topic, `bun nv peek` reads several in one call, `grep -n` answers the rest.
One minute there beats three rounds of revision.

This binds an answer given in conversation. [session-prompt.md](session-prompt.md) owns what a loop
session owes its goal; [doc-style.md](doc-style.md) owns how anything under `docs/` is written.

## The rules

1. **Every claim carries its evidence** — a `file:line`, a command's output, or the words *not
   checked* / *inferred*. Nothing sits between those.
2. **Before a sentence goes out, ask whether you would check it if pushed back on.** If yes, check it
   first.
3. **Advice nobody asked for meets the same bar as the answer.** It is where unverified claims
   collect, and it is the part a reader is most likely to act on.
4. **A proposal answers three questions first:** does it already exist here, is it implementable as
   described, and **which code path does it actually cover?** Skipping the third is how a check lands
   on a path the code never takes.
5. **Bring one plan, not a menu.** Ranked options multiply what must be verified and hide that none of
   it was. Name the rejected alternative in a sentence.
6. **"I have not checked" is a complete answer.** Say what finding out would cost and let the asker
   spend the call.
7. **When you do revise, say what still stands.** Churn in one corner otherwise reads as churn
   everywhere, and sound conclusions get discarded with the rest.
8. **End when the answer ends.** No trailing notes about state the asker can already see — what was
   committed, what is running, what is untracked. If a note would not change what they do next, cut
   it.
9. **Never hand back a decision you have already made.** Flagging something "rather than deciding" it,
   when you hold a view, is hedging dressed as thoroughness. Give the view.
10. **Default to short, and let the question set the depth.** A simple question gets a simple answer;
    go long only where the subject needs it or the asker asked for it. Length is not thoroughness —
    an answer skimmed because it is long transfers less than a short one read whole, and a follow-up
    question is cheap.

## Not this

Never hedge a claim you did verify — that buries a checked fact among the guesses. State it plainly
with its citation, and hedge only what earns it.
