bcrypt hashes at most 72 bytes of a password and stops at a NUL byte. Verification replicates that
exactly, because it must accept precisely the passwords the other implementation accepted against the
same stored row — it is the algorithm's behaviour, not a choice being made here.

Replicating it is the correct answer and an uncomfortable one, which is why it is written down: a
password longer than the limit has a suffix that never mattered, and two passwords sharing their first
72 bytes both verify. That is one more reason every such row is marked for rehash under an algorithm
without the edge (`rule:security/needs-rehash-answers-weaker`).
