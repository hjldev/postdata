import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  newRequest,
  parseParams,
  applyParams,
  type RequestSpec,
  type Workspace,
  type ResponseData,
} from "./types";
const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));
export const useWorkspace = defineStore("workspace", () => {
  const request = ref(newRequest());
  const data = ref<Workspace>({ version: 1, favorites: [], history: [] });
  const response = ref<ResponseData | null>(null);
  const busy = ref(false),
    error = ref(""),
    notice = ref(""),
    favoriteId = ref<string | null>(null);
  const desktop = isTauri();
  let activeId: string | null = null;
  async function refresh() {
    if (desktop) data.value = await invoke<Workspace>("load_workspace");
  }
  async function init() {
    try {
      await refresh();
    } catch (e) {
      error.value = String(e);
    }
  }
  function setUrl(url: string) {
    request.value.url = url;
    request.value.params = parseParams(url);
  }
  function syncParams() {
    request.value.url = applyParams(request.value.url, request.value.params);
  }
  function reset() {
    if (busy.value) return;
    request.value = newRequest();
    response.value = null;
    error.value = "";
    notice.value = "";
    favoriteId.value = null;
  }
  async function restore(value: RequestSpec, favorite = false) {
    if (busy.value) return;
    request.value = clone(value);
    favoriteId.value = favorite ? value.id : null;
    response.value = null;
    error.value = "";
    notice.value = "";
    if (desktop && value.body.kind === "multipart") {
      const paths = value.body.fields
        .filter((f) => f.kind === "file" && f.enabled && f.value)
        .map((f) => f.value);
      try {
        const missing = await invoke<string[]>("check_files", { paths });
        if (missing.length)
          notice.value = "以下文件已不可读，请重新选择：" + missing.join("、");
      } catch (e) {
        error.value = String(e);
      }
    }
  }
  async function send() {
    if (busy.value) return;
    error.value = "";
    notice.value = "";
    response.value = null;
    if (!desktop) {
      error.value = "请在 Postdata 桌面应用中发送请求。浏览器预览仅展示界面。";
      return;
    }
    if (!request.value.url.trim()) {
      error.value = "请输入 HTTP 或 HTTPS 请求地址";
      return;
    }
    const snapshot = clone(request.value);
    snapshot.id = crypto.randomUUID();
    snapshot.url = snapshot.url.trim();
    activeId = snapshot.id;
    busy.value = true;
    try {
      const outcome = await invoke<{
        response: ResponseData | null;
        error: string | null;
        storageError: string | null;
      }>("send_request", { request: snapshot });
      if (activeId === snapshot.id) {
        response.value = outcome.response;
        error.value = outcome.error || "";
        notice.value = outcome.storageError || "";
      }
      await refresh();
    } catch (e) {
      error.value = String(e);
    } finally {
      if (activeId === snapshot.id) {
        activeId = null;
        busy.value = false;
      }
    }
  }
  async function cancel() {
    try {
      if (activeId) await invoke("cancel_request", { id: activeId });
    } catch (e) {
      error.value = String(e);
    }
  }
  async function save(name: string, asNew = false) {
    if (!desktop) {
      error.value = "请在桌面应用中保存收藏";
      return;
    }
    try {
      const saved = clone(request.value);
      saved.name = name.trim() || saved.url || "未命名请求";
      saved.id =
        !asNew && favoriteId.value ? favoriteId.value : crypto.randomUUID();
      await invoke("save_favorite", { request: saved });
      favoriteId.value = saved.id;
      request.value.name = saved.name;
      await refresh();
      notice.value = "请求已保存到收藏";
    } catch (e) {
      error.value = String(e);
    }
  }
  async function remove(id: string) {
    try {
      await invoke("delete_favorite", { id });
      if (favoriteId.value === id) favoriteId.value = null;
      await refresh();
    } catch (e) {
      error.value = String(e);
    }
  }
  async function clearHistory() {
    try {
      await invoke("clear_history");
      await refresh();
    } catch (e) {
      error.value = String(e);
    }
  }
  return {
    request,
    data,
    response,
    busy,
    error,
    notice,
    favoriteId,
    desktop,
    init,
    setUrl,
    syncParams,
    reset,
    restore,
    send,
    cancel,
    save,
    remove,
    clearHistory,
  };
});
