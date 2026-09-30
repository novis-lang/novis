A key that ends in `_file` reads a secret, such as a password, from a separate file, so the secret
is not written in `nvs.toml`.

Two settings have this form today: `password_file` in `[db.<name>]` and in `[mail.<name>]`. The
whole content of the file is the value. Novis removes one line break at the end and changes nothing
else. Set either `password` or `password_file`. Setting both is the error `E0608`.

Novis reads the file when the server starts and each time the configuration is applied.
`nvs config dump` prints `<secret>` and the name of the file, and never the value.

Novis decrypts nothing. Tools such as SOPS, Docker Compose, Kubernetes and Vault all produce a
plain file, and `password_file` names that file. Decrypt the file before the server starts.

**Good to know:** the server does not start when another account can write to the secret file. It
prints a warning when another account can read it. A password is used exactly as written. A space
at its start or end is kept, and Novis prints the warning `W1007`.
