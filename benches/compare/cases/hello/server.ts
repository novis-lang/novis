// Returns one line for every request. This measures the server alone.
export default {
  fetch(): Response {
    return new Response("hello, world\n", { headers: { "Content-Type": "text/html; charset=UTF-8" } });
  },
};
