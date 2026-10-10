// Returns the version of each of the 200 parts. The driver writes the parts and later replaces them.
const parts: { version: string }[] = await Promise.all(
  Array.from({ length: 200 }, (_, i) => import(`./parts/p${String(i).padStart(3, "0")}.ts`)),
);

export default {
  fetch(): Response {
    return new Response(`${parts.map((p) => p.version).join("")}\n`, { headers: { "Content-Type": "text/html; charset=UTF-8" } });
  },
};
