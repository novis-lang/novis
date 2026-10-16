Names which database a connection speaks to.

It is the first thing a connection's settings say, and it decides what the rest of those settings have
to be: the four server backends are reached at a host, while SQLite is a file you give a path to. There
is one case for each backend Novis can connect to — MySQL, MariaDB, PostgreSQL, SQLite and SQL Server.
MariaDB is its own case rather than a setting on MySQL, because its logins and its error codes are its
own.

**Good to know:** there is no case meaning "whichever database is configured". A connection can be
asked which driver it is on, and a program only asks where the answer changes what it does — the
handful of places where one backend wants different SQL from another. Everything else is written once
and runs on all five.
