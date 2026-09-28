`Core\Server::isDraining()` returns `true` when the server that runs your program has started to shut
down. It returns `false` while the server works normally.

A server shuts down in steps. First it stops accepting new connections. Then it gives the requests it
already has some time to finish. This is called draining. During this time, `isDraining()` returns
`true`, so a request can see that the server is about to stop.

A command-line program is not run by a server. There, `isDraining()` always returns `false`.

**Good to know:** the server's `health_path` setting gives a load balancer the same answer. Use
`isDraining()` when you write your own health check or when a long request should stop early.

**The examples below** show the answer in a command-line program, a health check for a load balancer,
and a job that stops between two items when the server shuts down.
