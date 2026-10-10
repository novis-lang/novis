// Returns the value a module variable has when the request starts, then the value from `?v=`.
// A request that sees `0` first did not see the value an earlier request wrote.
let value = 0;

export default {
  fetch(req: Request): Response {
    const seen = value;
    value = Number(new URL(req.url).searchParams.get("v") ?? "0");
    return new Response(`${seen} ${value}`, { headers: { "Content-Type": "text/html; charset=UTF-8" } });
  },
};
