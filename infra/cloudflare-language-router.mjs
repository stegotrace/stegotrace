export function preferredLanguage(header = "") {
  const accepted = header
    .split(",")
    .map((entry, order) => {
      const [tag, ...parameters] = entry.trim().toLowerCase().split(";");
      const quality = parameters
        .map((parameter) => parameter.trim())
        .find((parameter) => parameter.startsWith("q="));
      return { tag, order, quality: quality ? Number(quality.slice(2)) : 1 };
    })
    .filter(({ quality }) => Number.isFinite(quality) && quality > 0)
    .sort((left, right) => right.quality - left.quality || left.order - right.order);

  for (const { tag } of accepted) {
    if (tag === "es" || tag.startsWith("es-")) return "es";
    if (tag === "en" || tag.startsWith("en-")) return "en";
  }
  return "es";
}

export default {
  async fetch(request) {
    const url = new URL(request.url);
    if (url.pathname !== "/") return fetch(request);

    url.pathname = `/${preferredLanguage(request.headers.get("accept-language") ?? "")}/`;
    return new Response(null, {
      status: 302,
      headers: {
        location: url.toString(),
        vary: "Accept-Language",
        "cache-control": "private, no-store",
      },
    });
  },
};
