# Prose

All prose is English: docs, agent docs, comments, commit bodies, and the text of the UI and the CLI.
`bun nv prose` checks the shape of the files a change touched, and never their meaning.
The rules below bind text an end user reads only.

- **A sentence has at most 25 words, and one idea.** Split a longer one where its second idea starts.
- **No dash joins two sentences.** Write two sentences, or use a colon when the second explains the
  first.
- **No idioms.** Say the plain thing: "start", not "kick off".
- **Use the words a programmer already knows.** A name the reader types goes in backticks.
- **A term is defined where it first appears.**
- **Text an end user reads names no internals:** no decision numbers and no goal slugs.
- **No filler.** Cut "simply", "just", "basically", "note that" and "in order to".

`bun nv prose` flags each quoted word on the *No filler* line above. `looper prose` runs the same
command through `looper.toml`'s `[gates] prose`. AGENTS.md § *Text an end user reads* is the whole rule
for this project's wording.

## A newcomer reads it

Every gate stage starts with this review, before its first `looper verify`. It is the **Reviewed**
proof of `rule:testing/feature-proofs`, and `tools/nv/lib/texts.ts` says what a user text is.

1. **List.** `bun nv site --review` prints every text that is not reviewed, one key per line, under the
   feature it belongs to. These are the texts the goal changed.
2. **Batch.** Split the keys into batches of at most 20 texts, keeping a feature's texts together.
   `bun nv site --review --brief <key>...` prints one batch's brief: the rules above, the three
   questions, and each text with its key, a help card as `nvs agent show` prints it.
3. **Read.** Send each brief to a `reader` subagent, several at once. The prompt is the brief, and it
   tells the reader to read no file and answer from the brief alone. For each text the reader reports:
   1. each sentence it had to read twice, quoted;
   2. each word or term it did not know;
   3. in its own words, in two or three sentences, what the feature does, or what the page teaches.
4. **Compare.** The session checks each answer to (3) against the feature's `about.md`, or against what
   the page covers. A finding is a pointer to check, not a verdict.
5. **Fix.** Every finding is fixed in the text. A wrong (3) is fixed by rewriting the text that misled
   the reader, never by explaining the reader's mistake somewhere else.
6. **Stamp.** Once every finding of a text is fixed, `bun nv site --review --stamp <key>...` records
   its hash. A stamp is never written to quiet `bun nv site --check review`, and a text rewritten after
   its stamp owes a new review.
