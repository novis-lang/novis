Returns one value from the `[secrets]` block of `nvs.toml` as a `secret string`, or `null` if no block sets it.

Use it for a password, an API key or a token. A secret can go into an outbound HTTP header, a form
field, a JSON body and a database password. `echo`, `print`, `Core\Log::write` and a response body
do not accept it. A program that tries does not compile.

`api_key = "…"` writes the value in `nvs.toml`. `api_key_file = "api_key.txt"` reads it from a file.
A relative path starts at the folder of `nvs.toml`. One newline at the end of the file is removed.

**In plain words:** a secret is a value your program can use but never show. The name must be text
your program wrote. A name from a request does not compile, so a visitor cannot choose the secret.

**Good to know:** `[app.secrets]` under `[[app]]` replaces the `[secrets]` value with the same name.
`Core\Config::get` and `Core\Config::all` never return a secret. `nvs` prints a warning when other
accounts can read a secret file, so make the file readable only by the account that runs `nvs`.
