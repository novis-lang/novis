Returns the value of one environment variable as a `secret string`, or `null` if nothing set it.

Use it for a password, an API key or a token. A secret can go into an outbound HTTP header, a form
field, a JSON body and a database password. `echo`, `print`, `Core\Log::write` and a response body
do not accept it. A program that tries does not compile, so the value cannot reach a page or a log
by mistake.

**In plain words:** a secret is a value your program can use but never show. `Core\Env::get` reads
the same variable as a tainted string. Use `secret` when the value is a credential.

The value is not tainted. The name must be text your program wrote itself. A name that came from a
request does not compile, because a visitor could then choose which secret the program reads.

**Good to know:** if the value is not valid UTF-8 text, `secret` throws a `RuntimeError`. The
message has the name of the variable, and never the value.
