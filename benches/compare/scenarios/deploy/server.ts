// Returns the version of each part. The driver writes the parts and later replaces them.
// `DEPLOY_PARTS` is the number of parts, 200 unless the row sets it.
const count = Number(process.env.DEPLOY_PARTS || 200);
const digits = String(count - 1).length;
const parts: { version: string }[] = await Promise.all(
  Array.from({ length: count }, (_, i) => import(`./parts/p${String(i).padStart(digits, "0")}.ts`)),
);

export default {
  fetch(): Response {
    return new Response(`${parts.map((p) => p.version).join("")}\n`, { headers: { "Content-Type": "text/html; charset=UTF-8" } });
  },
};
