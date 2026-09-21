import http from "node:http";
// Local-only echo endpoint for manual desktop acceptance. No external services needed.
const server = http.createServer(async (req, res) => {
  if (req.url.startsWith("/slow"))
    await new Promise((resolve) => setTimeout(resolve, 5000));
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  res.setHeader("Content-Type", "application/json; charset=utf-8");
  res.setHeader("X-Postdata-Test", "local-echo");
  if (req.url.startsWith("/cookies"))
    res.setHeader("Set-Cookie", [
      "demo_session=postdata; Path=/; HttpOnly",
      "demo_saved=hello; Path=/; Max-Age=3600",
    ]);
  res.end(
    JSON.stringify(
      {
        message: "Hello from Postdata",
        method: req.method,
        url: req.url,
        headers: req.headers,
        body: Buffer.concat(chunks).toString(),
        local: true,
      },
      null,
      2,
    ),
  );
});
server.listen(4318, "127.0.0.1", () =>
  console.log("Postdata test server: http://127.0.0.1:4318/echo"),
);
