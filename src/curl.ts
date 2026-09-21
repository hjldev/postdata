import type { RequestSpec } from "./types";

/** Quote one POSIX shell argument without expanding $, backticks, or single quotes. */
export function shellQuote(value: string): string {
  if (value.includes("\0")) throw new Error("cURL 命令不支持 NUL 字符");
  return "'" + value.replaceAll("'", "'\"'\"'") + "'";
}

export function generateCurl(request: RequestSpec, cookieHeader = ""): string {
  let url: URL;
  try {
    url = new URL(request.url.trim());
  } catch {
    throw new Error("请先填写有效的 HTTP 或 HTTPS URL");
  }
  if (!["http:", "https:"].includes(url.protocol))
    throw new Error("仅支持 HTTP 和 HTTPS URL");
  url.hash = "";
  const lines = ["curl"];
  if (request.method === "HEAD") {
    if (request.body.kind !== "none")
      throw new Error("cURL 的 HEAD 模式不支持请求体，请将请求体设为“无”");
    lines.push("  --head");
  } else {
    lines.push(`  --request ${shellQuote(request.method)}`);
  }
  lines.push(`  --url ${shellQuote(url.toString())}`);
  const headers = request.headers.filter(
    (h) =>
      h.enabled &&
      h.key &&
      !(
        request.auth.kind !== "none" && h.key.toLowerCase() === "authorization"
      ) &&
      !(
        request.body.kind === "multipart" &&
        h.key.toLowerCase() === "content-type"
      ),
  );
  const hasHeader = (name: string) =>
    headers.some((h) => h.key.toLowerCase() === name);
  for (const h of headers) {
    if (!/^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/.test(h.key) || /[\r\n]/.test(h.value))
      throw new Error("请求头名称或值无效");
    // A semicolon asks curl to send an empty header instead of removing it.
    lines.push(
      `  --header ${shellQuote(h.value === "" ? `${h.key};` : `${h.key}: ${h.value}`)}`,
    );
  }
  if (request.auth.kind === "bearer")
    lines.push(
      `  --header ${shellQuote("Authorization: Bearer " + request.auth.token)}`,
    );
  if (request.auth.kind === "basic")
    lines.push(
      "  --basic",
      `  --user ${shellQuote(request.auth.username + ":" + request.auth.password)}`,
    );
  if (!hasHeader("cookie") && cookieHeader)
    lines.push(`  --cookie ${shellQuote(cookieHeader)}`);
  const contentType: Record<string, string> = {
    json: "application/json",
    text: "text/plain; charset=utf-8",
    form: "application/x-www-form-urlencoded",
  };
  if (!hasHeader("content-type") && contentType[request.body.kind])
    lines.push(
      `  --header ${shellQuote("Content-Type: " + contentType[request.body.kind])}`,
    );
  switch (request.body.kind) {
    case "json":
      try {
        JSON.parse(request.body.text);
      } catch {
        throw new Error("请求体不是有效的 JSON");
      }
      lines.push(`  --data-raw ${shellQuote(request.body.text)}`);
      break;
    case "text":
      lines.push(`  --data-raw ${shellQuote(request.body.text)}`);
      break;
    case "form": {
      const form = new URLSearchParams();
      request.body.fields
        .filter((f) => f.enabled && f.key)
        .forEach((f) => form.append(f.key, f.value));
      lines.push(`  --data-raw ${shellQuote(form.toString())}`);
      break;
    }
    case "multipart":
      for (const field of request.body.fields.filter(
        (f) => f.enabled && f.key,
      )) {
        if (/[=\r\n]/.test(field.key))
          throw new Error("cURL 表单字段名不能包含等号或换行");
        if (field.kind === "file") {
          if (!field.value) throw new Error(`请为 ${field.key} 选择上传文件`);
          const path = field.value
            .replaceAll("\\", "\\\\")
            .replaceAll('"', '\\"');
          lines.push(`  --form ${shellQuote(`${field.key}=@"${path}"`)}`);
        } else
          lines.push(
            `  --form-string ${shellQuote(`${field.key}=${field.value}`)}`,
          );
      }
      break;
  }
  return lines.join(" \\\n");
}
