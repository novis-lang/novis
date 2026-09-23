- **A goal's number is its position, so inserting one renumbers every goal after it — and that is
  fine only because nothing but the tool writes a number.** `python tools/chain.py --new <slug>
  --after N` renames the files it displaces and rewrites the two headers and the link targets that
  carry a number; prose names a goal by its slug and is untouched. Never rename a goal file by hand,
  and never write `goal 29` — nor a *range* of them, `goals 30–44`, which is the form that got past
  this and into the handoff — into a sentence; `chain.py --check` fails on it, because the next
  insert would silently make it name a different goal. [until: gone tools/chain.py:number_citations]
