- **A `-p nvs-server` test can drive a real upgrade handshake, and one connection carries the
  upgradable request and an ordinary one after it.** `hyper` keeps a connection whose `Connection:
  Upgrade` request was answered `200`: the response is framed normally and keep-alive continues.
  `only_an_upgradable_request_is_offered_a_slot` is the shape, and
  `request.extensions().get::<hyper::upgrade::OnUpgrade>()` answers "can this connection be
  upgraded". [until: gone crates/nvs-server/src/serve.rs:only_an_upgradable_request_is_offered_a_slot]
