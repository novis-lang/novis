`Core\Net` has no member that opens a connected pair of anonymous sockets, and PHP's
`stream_socket_pair` is a `dropped` row. A socket pair exists to be handed to a forked child. Novis has
no fork, and the one process boundary it has is an isolate, which shares nothing
(`rule:security/isolate-shares-nothing`) and so cannot receive a descriptor. A member for it would have
no caller, and a PHP function with no Novis answer is a `dropped` row, never a member written to fill a
hole.
