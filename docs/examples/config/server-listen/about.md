The addresses where the server accepts connections.

Each entry is an address and a port, such as `127.0.0.1:8000`, or the full path of a Unix socket.
When the file does not set it, the server listens on `127.0.0.1`, port `8000`. Unix sockets work
only on Unix systems, such as Linux and macOS. On Windows, the server listens on TCP.

A program cannot change this setting. The server reads it only when it starts. If you change it in
the file, the server keeps its old addresses, and `nvs ctl status` shows the new value until you
restart the server.

**Good to know:** a port below 1024 needs special permissions. The server gives them up after it
starts, so it cannot move to such a port while it runs.
