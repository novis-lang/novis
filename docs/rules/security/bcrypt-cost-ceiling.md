A stored bcrypt hash carries its own cost, and the cost is the base-2 logarithm of the round count — a
CPU-time lever one `UPDATE` away from anyone who can write the table. Verification refuses a stored
cost above **17** before doing any work, as an ordinary throw.

Seventeen is far above what real deployments write and far below the ceiling's minutes of CPU per
verification, so it costs no real table anything and closes the denial of service. Refusing *before*
any work is what makes it a defence rather than a report: a check made afterwards has already spent
the CPU the attacker was buying.

The same shape guards the modern algorithm's memory parameter, which is the reason this is a ceiling
on a stored value rather than a validation of a written one.
