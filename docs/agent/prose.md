<!-- looper:begin prose v2 -->
# Prose

All prose is English: docs, agent docs, comments, commit bodies, and the text of the UI and the CLI.
`looper prose` checks the shape of the files a change touched, and never their meaning.
The rules below bind text an end user reads only.

- **A sentence has at most 25 words, and one idea.** Split a longer one where its second idea starts.
- **No dash joins two sentences.** Write two sentences, or use a colon when the second explains the
  first.
- **No idioms.** Say the plain thing: "start", not "kick off".
- **Use the words a programmer already knows.** A name the reader types goes in backticks.
- **A term is defined where it first appears.**
- **Text an end user reads names no internals:** no decision numbers and no goal slugs.
- **No filler.** Cut "simply", "just", "basically", "note that" and "in order to".

`looper.toml`'s `[prose] banned` adds this project's own words to the ones `looper prose` flags.
