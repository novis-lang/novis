# Novis\Rest

`Novis\Rest` is a client for OAuth 2 servers. It requests access tokens, finds a server's
endpoints, and signs DPoP proofs. It also sends requests with a token.

## What it has

| Class | What it does |
|---|---|
| `Client` | An OAuth 2 client: its id, its secret, and the server's token endpoint. It sends requests with a token. |
| `Token` | An access token, its type, its expiry, its scope and its refresh token. |
| `ClientCredentials` | Gets a token for a server program with its own id and secret. |
| `AuthorizationCode` | Signs in a user in the browser with PKCE, and refreshes the token. |
| `Discovery` | Reads a server's metadata document and finds its endpoints. |
| `DPoP` | Signs a DPoP proof for each request, so a stolen token cannot be used alone. |
| `Session` | Keeps a signed-in user's tokens in the user's session, encrypted. |

## An example

```nvs
<?nvs
Novis\Rest\Discovery $server = Novis\Rest\Discovery::fetch('https://login.example.com');
secret string $clientSecret = Core\Env::secret('SHOP_CLIENT_SECRET') ?? '';
Novis\Rest\Client $client = $server->client('shop', $clientSecret);

// `token` keeps the token in memory until 30 seconds before it expires.
// `$keys` encrypt the token in memory. `Core\Crypto::generateKey()` creates one.
array<secret bytes> $keys = [Core\Crypto::generateKey()];
Novis\Rest\Token $token = Novis\Rest\ClientCredentials::token($client, 'orders.read', $keys);
Core\Http\Response $orders = $client->get('https://api.example.com/orders', $token);
```

## Capabilities

The package needs `net.connect`, because it sends requests to the OAuth server.

## License

MIT. See `LICENSE`.
