import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createServer } from "node:http";
import { spawn } from "node:child_process";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { generateCurl, shellQuote } from "./curl";
import { newRequest, pair, type RequestSpec } from "./types";

const server = createServer(async (req, res) => {
  const chunks: Buffer[] = [];
  for await (const chunk of req) chunks.push(Buffer.from(chunk));
  res.setHeader("content-type", "application/json");
  res.setHeader("x-echo-method", req.method || "");
  res.end(
    JSON.stringify({
      method: req.method,
      url: req.url,
      headers: req.headers,
      body: Buffer.concat(chunks).toString(),
    }),
  );
});
let base = "";
beforeAll(async () => {
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("No test port");
  base = `http://127.0.0.1:${address.port}`;
});
afterAll(async () => {
  await new Promise<void>((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
});
function run(command: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn("/bin/sh", ["-c", command], {
      stdio: ["ignore", "pipe", "pipe"],
      env: {
        ...process.env,
        NO_PROXY: "127.0.0.1,localhost",
        no_proxy: "127.0.0.1,localhost",
      },
    });
    let stdout = "",
      stderr = "";
    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("error", reject);
    child.on("exit", (code) =>
      code === 0 ? resolve(stdout) : reject(new Error(stderr)),
    );
  });
}
function request(): RequestSpec {
  return { ...newRequest(), url: `${base}/echo?a=1&a=2#fragment` };
}
const field = (key: string, value: string, enabled = true, kind = "text") => ({
  ...pair(),
  key,
  value,
  enabled,
  kind,
});
const execute = async (r: RequestSpec, cookie = "") =>
  JSON.parse(await run(generateCurl(r, cookie)));

describe("cURL export executed against a local HTTP endpoint", () => {
  it("keeps method and repeated URL params without appending twice", async () => {
    const r = request();
    r.method = "PATCH";
    r.params = [field("a", "1"), field("a", "2")];
    const result = await execute(r);
    expect(result.method).toBe("PATCH");
    expect(result.url).toBe("/echo?a=1&a=2");
  });
  it("preserves raw text and prevents shell expansion", async () => {
    const r = request();
    r.method = "POST";
    r.body.kind = "text";
    r.body.text =
      "@not-a-file\nO'Reilly $(printf hacked) `printf nope` $HOME 中文\\";
    const result = await execute(r);
    expect(result.body).toBe(r.body.text);
    expect(result.headers["content-type"]).toBe("text/plain; charset=utf-8");
  });
  it("preserves JSON and generates the effective Bearer header", async () => {
    const r = request();
    r.method = "POST";
    r.body.kind = "json";
    r.body.text = JSON.stringify({ name: "O'Reilly", flag: true }, null, 2);
    r.auth.kind = "bearer";
    r.auth.token = "demo-token";
    r.headers = [
      field("authorization", "overridden"),
      field("X-Repeat", "one"),
      field("X-Repeat", "two"),
      field("X-Off", "hidden", false),
      field("X-Empty", ""),
    ];
    const result = await execute(r, "session=demo");
    expect(result.body).toBe(r.body.text);
    expect(result.headers.authorization).toBe("Bearer demo-token");
    expect(result.headers["x-repeat"]).toBe("one, two");
    expect(result.headers["x-off"]).toBeUndefined();
    expect(result.headers["x-empty"]).toBe("");
    expect(result.headers.cookie).toBe("session=demo");
  });
  it("keeps Basic auth and gives manual Cookie precedence", async () => {
    const r = request();
    r.auth = {
      kind: "basic",
      username: "tester",
      password: "p'ass:$word",
      token: "",
    };
    r.headers = [field("Cookie", "manual=yes")];
    const result = await execute(r, "session=ignored");
    expect(result.headers.authorization).toBe(
      "Basic " + Buffer.from("tester:p'ass:$word").toString("base64"),
    );
    expect(result.headers.cookie).toBe("manual=yes");
  });
  it("encodes repeated form keys, Unicode, empty values and disabled rows", async () => {
    const r = request();
    r.method = "POST";
    r.body.kind = "form";
    r.body.fields = [
      field("a", "1"),
      field("a", "2"),
      field("q", "中文 +&="),
      field("empty", ""),
      field("off", "no", false),
    ];
    const result = await execute(r);
    expect([...new URLSearchParams(result.body)]).toEqual([
      ["a", "1"],
      ["a", "2"],
      ["q", "中文 +&="],
      ["empty", ""],
    ]);
  });
  it("uploads files with spaces, quotes, commas and semicolons in their paths", async () => {
    const dir = await mkdtemp(join(tmpdir(), "postdata-curl-"));
    try {
      const path = join(dir, "file \",;'name.txt");
      await writeFile(path, "upload-content");
      const r = request();
      r.method = "POST";
      r.body.kind = "multipart";
      r.headers = [field("Content-Type", "incorrect-boundary")];
      r.body.fields = [
        field("note", "@literal;type=text/plain"),
        field("upload", path, true, "file"),
      ];
      const result = await execute(r);
      expect(result.headers["content-type"]).toMatch(
        /^multipart\/form-data; boundary=/,
      );
      expect(result.body).toContain("upload-content");
      expect(result.body).toContain("@literal;type=text/plain");
      expect(result.body).toContain('name="note"');
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });
  it("uses HEAD mode without exporting application transport options", async () => {
    const r = request();
    r.method = "HEAD";
    r.timeoutMs = 1250;
    const command = generateCurl(r);
    expect(command).toContain("--head");
    for (const option of [
      "--disable",
      "--globoff",
      "--noproxy",
      "--location",
      "--max-redirs",
      "--max-time",
      "--cookie",
    ]) {
      expect(command).not.toContain(option);
    }
    expect(await run(command)).toContain("x-echo-method: HEAD");
  });
  it("rejects unsupported or unrepresentable input instead of misleading commands", () => {
    const r = request();
    r.url = "file:///etc/hosts";
    expect(() => generateCurl(r)).toThrow("HTTP");
    expect(() => shellQuote("a\0b")).toThrow("NUL");
    r.url = base;
    r.body.kind = "json";
    r.body.text = "{";
    expect(() => generateCurl(r)).toThrow("JSON");
  });
});
