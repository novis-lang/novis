One configuration split across more than one file: an `[[include]]` entry names another file, or a
directory of them, and what that file says is read in place.

Most deployments have two sets of settings — the ones that belong in version control, and the ones
that belong to this machine or this environment: a database address, a key, a proxy. An include lets
the second set live in its own file, and an entry naming a directory reads every `.toml` directly
inside it in filename order, so dropping a file in is how a host adds its own settings without
editing anything that is shared. Any file in the tree may set any directive, and where two of them
say different things the later one wins.

An entry names a file or a directory, never both, and may say that an absent file is fine.
Everything else stays strict: a file that exists but cannot be read, cannot be parsed, or could be
rewritten by an account other than the server's stops the server rather than being quietly skipped.
