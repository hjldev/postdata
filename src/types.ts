export interface Pair {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
  kind: string;
}
export interface RequestSpec {
  id: string;
  name: string;
  method: string;
  url: string;
  params: Pair[];
  headers: Pair[];
  auth: { kind: string; token: string; username: string; password: string };
  body: { kind: string; text: string; fields: Pair[] };
  timeoutMs: number;
}
export interface ResponseData {
  status: number;
  finalUrl: string;
  headers: [string, string][];
  elapsedMs: number;
  size: number;
  text: string | null;
  binary: boolean;
  truncated: boolean;
}
export interface History {
  id: string;
  at: number;
  request: RequestSpec;
  status: number | null;
  elapsedMs: number;
  error: string | null;
}
export interface Workspace {
  version: number;
  favorites: RequestSpec[];
  history: History[];
}
export const pair = (): Pair => ({
  id: crypto.randomUUID(),
  key: "",
  value: "",
  enabled: true,
  kind: "text",
});
export const newRequest = (): RequestSpec => ({
  id: crypto.randomUUID(),
  name: "",
  method: "GET",
  url: "",
  params: [],
  headers: [],
  auth: { kind: "none", token: "", username: "", password: "" },
  body: { kind: "none", text: "", fields: [] },
  timeoutMs: 30000,
});
export function parseParams(url: string): Pair[] {
  const query = url.split("#")[0].split("?").slice(1).join("?");
  return [...new URLSearchParams(query)].map(([key, value]) => ({
    ...pair(),
    key,
    value,
  }));
}
export function applyParams(url: string, params: Pair[]): string {
  const hashIndex = url.indexOf("#");
  const hash = hashIndex >= 0 ? url.slice(hashIndex) : "";
  const base = (hashIndex >= 0 ? url.slice(0, hashIndex) : url).split("?")[0];
  const query = new URLSearchParams();
  params
    .filter((p) => p.enabled && p.key)
    .forEach((p) => query.append(p.key, p.value));
  return base + (query.size ? "?" + query.toString() : "") + hash;
}

export interface CookieInfo {
  name: string;
  value: string;
  domain: string;
  path: string;
  secure: boolean;
  httpOnly: boolean;
  expiresAt: number | null;
}
