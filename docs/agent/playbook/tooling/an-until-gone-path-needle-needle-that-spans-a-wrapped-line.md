- **An `[until: gone <path>:<needle>]` needle that spans a wrapped line retires itself the first
  time `bun nv session --wrap` runs.** The needle is a plain substring of the file, so a sentence
  broken across a string continuation or a comment wrap holds it nowhere, and the bullet is deleted
  with a message stating the file no longer holds a sentence it plainly still does. Grep the needle
  before writing the trailer, and shorten it to the longest run that sits on one line.
  [until: gone tools/nv/cmd/playbook.ts:A needle is a plain substring]
