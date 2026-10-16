`Core\SignedCookie::seal` stores a value in a cookie. The browser keeps the cookie, but it cannot
read or change the value. `seal` encrypts the value with a secret key and returns text that you put
in a `Set-Cookie` header. When the browser sends the cookie back, `Core\SignedCookie::open` checks
it and returns the value.

You give `seal` a list of keys, newest first, and the newest key encrypts the value. After you add
a new key, keep the old one in the list for a while. Then cookies made with the old key still work.

Each call returns different text, even for the same value. The cookie is about 4/3 × (the length of
the value + 40) characters long. A browser keeps about 4096 characters per cookie, so store small
values, such as a user id.

**The examples below** seal and open a value, show that the text changes on each call, and keep a
shopping cart in a cookie.
