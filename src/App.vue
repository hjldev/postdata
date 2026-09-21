<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowUpRight,
  Plus,
  Search,
  Star,
  History as HistoryIcon,
  Send,
  Square,
  Cookie,
  X,
  Trash2,
  Copy,
  Check,
  Clock3,
  Globe2,
  Braces,
  ArrowDown,
  Save,
  Command,
  FolderOpen,
  Terminal,
} from "lucide-vue-next";
import type { CookieInfo } from "./types";
import { copyText } from "./clipboard";
import { generateCurl } from "./curl";
import { useWorkspace } from "./store";
import PairEditor from "./components/PairEditor.vue";
import CodeEditor from "./components/CodeEditor.vue";
const store = useWorkspace();
const sidebar = ref("favorites"),
  search = ref(""),
  tab = ref("params"),
  responseTab = ref("body"),
  pretty = ref(true);
const saveOpen = ref(false),
  saveName = ref(""),
  cookieOpen = ref(false),
  cookies = ref<CookieInfo[]>([]),
  cookieWarning = ref(""),
  copied = ref(false);
const curlOpen = ref(false),
  curlText = ref(""),
  curlError = ref(""),
  curlCopied = ref(false),
  curlLoading = ref(false);
async function showCurl() {
  curlOpen.value = true;
  curlLoading.value = true;
  curlText.value = "";
  curlError.value = "";
  curlCopied.value = false;
  try {
    // Validate before invoking the native Cookie lookup; generating never sends the request.
    generateCurl(store.request);
    const manualCookie = store.request.headers.some(
      (h) => h.enabled && h.key.toLowerCase() === "cookie",
    );
    const cookieHeader =
      store.desktop && !manualCookie
        ? await invoke<string>("request_cookie_header", {
            url: store.request.url,
          })
        : "";
    curlText.value = generateCurl(store.request, cookieHeader);
  } catch (error) {
    curlError.value = String(error).replace(/^Error: /, "");
  } finally {
    curlLoading.value = false;
  }
}
async function copyCurl() {
  try {
    await copyText(curlText.value);
    curlCopied.value = true;
  } catch (error) {
    curlError.value = "复制失败：" + String(error);
  }
}
const favorites = computed(() =>
  store.data.favorites.filter((r) =>
    (r.name + r.url).toLowerCase().includes(search.value.toLowerCase()),
  ),
);
const history = computed(() =>
  store.data.history.filter((r) =>
    (r.request.name + r.request.url)
      .toLowerCase()
      .includes(search.value.toLowerCase()),
  ),
);
const responseText = computed(() => {
  const text = store.response?.text || "";
  if (pretty.value) {
    try {
      return JSON.stringify(JSON.parse(text), null, 2);
    } catch {}
  }
  return text;
});
const jsonResponse = computed(() => {
  try {
    JSON.parse(store.response?.text || "");
    return true;
  } catch {
    return false;
  }
});
const tabs = computed(() => [
  {
    id: "params",
    label: "参数",
    count: store.request.params.filter((p) => p.enabled && p.key).length,
  },
  {
    id: "headers",
    label: "请求头",
    count: store.request.headers.filter((p) => p.enabled && p.key).length,
  },
  {
    id: "body",
    label: "请求体",
    count: store.request.body.kind === "none" ? 0 : 1,
  },
  {
    id: "auth",
    label: "鉴权",
    count: store.request.auth.kind === "none" ? 0 : 1,
  },
  { id: "settings", label: "设置", count: 0 },
]);
function label(url: string) {
  try {
    const u = new URL(url);
    return u.pathname === "/" ? u.host : u.pathname;
  } catch {
    return url || "未命名请求";
  }
}
function size(bytes: number) {
  return bytes < 1024
    ? `${bytes} B`
    : bytes < 1048576
      ? `${(bytes / 1024).toFixed(1)} KB`
      : `${(bytes / 1048576).toFixed(1)} MB`;
}
function openSave() {
  saveName.value = store.request.name || label(store.request.url);
  saveOpen.value = true;
}
async function save(asNew = false) {
  await store.save(saveName.value, asNew);
  saveOpen.value = false;
}
async function loadCookies() {
  try {
    const result = await invoke<{
      cookies: CookieInfo[];
      warning: string | null;
    }>("list_cookies");
    cookies.value = result.cookies;
    cookieWarning.value = result.warning || "";
  } catch (e) {
    cookieWarning.value = String(e);
  }
}
async function showCookies() {
  cookieOpen.value = true;
  if (store.desktop) await loadCookies();
}
function cookieExpiry(c: CookieInfo) {
  return c.expiresAt === null
    ? "会话结束"
    : new Date(c.expiresAt * 1000).toLocaleString("zh-CN");
}
async function deleteCookie(c: CookieInfo) {
  try {
    await invoke("delete_cookie", {
      domain: c.domain,
      path: c.path,
      name: c.name,
    });
    await loadCookies();
  } catch (e) {
    cookieWarning.value = String(e);
  }
}
async function clearCookies() {
  try {
    await invoke("clear_cookies");
    await loadCookies();
  } catch (e) {
    cookieWarning.value = String(e);
  }
}
async function copyResponse() {
  try {
    await copyText(responseText.value);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1600);
  } catch (e) {
    store.error = "复制失败：" + String(e);
  }
}
function shortcut(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
    event.preventDefault();
    if (!saveOpen.value && !cookieOpen.value && !curlOpen.value) store.send();
  }
  if (event.key === "Escape") {
    saveOpen.value = false;
    cookieOpen.value = false;
    curlOpen.value = false;
  }
}
onMounted(() => {
  store.init();
  window.addEventListener("keydown", shortcut);
});
onBeforeUnmount(() => window.removeEventListener("keydown", shortcut));
</script>
<template>
  <div class="app-shell">
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-mark">
          <ArrowUpRight :size="24" :stroke-width="2.4" />
        </div>
        <strong>postdata<span>.</span></strong
        ><span class="version">BETA</span>
      </div>
      <button
        class="new-request"
        :disabled="store.busy"
        @click="
          store.reset();
          tab = 'params';
        "
      >
        <Plus :size="16" /> 新建请求 <span>＋</span>
      </button>
      <div class="side-switch">
        <button
          :class="{ active: sidebar === 'favorites' }"
          @click="sidebar = 'favorites'"
        >
          <Star :size="14" /> 收藏
          <span>{{ store.data.favorites.length }}</span></button
        ><button
          :class="{ active: sidebar === 'history' }"
          @click="sidebar = 'history'"
        >
          <HistoryIcon :size="14" /> 历史
        </button>
      </div>
      <div class="search-box">
        <Search :size="14" /><input
          v-model="search"
          placeholder="搜索请求…"
          aria-label="搜索请求"
        /><span>⌕</span>
      </div>
      <div class="list-caption">
        <span>{{ sidebar === "favorites" ? "我的请求" : "最近的请求" }}</span
        ><button
          v-if="sidebar === 'history' && store.data.history.length"
          class="icon-button"
          title="清空历史记录"
          @click="store.clearHistory()"
        >
          <Trash2 :size="13" /></button
        ><span v-else>{{ favorites.length }}</span>
      </div>
      <div class="request-list">
        <template v-if="sidebar === 'favorites'">
          <div
            v-for="item in favorites"
            :key="item.id"
            class="saved-row"
            :class="{ selected: store.favoriteId === item.id }"
          >
            <button
              class="saved-request"
              :disabled="store.busy"
              @click="store.restore(item, true)"
            >
              <span class="method-label" :class="item.method.toLowerCase()">{{
                item.method
              }}</span
              ><span>{{ item.name || label(item.url) }}</span></button
            ><button
              class="icon-button remove-saved"
              title="删除收藏"
              @click="store.remove(item.id)"
            >
              <X :size="12" />
            </button>
          </div>
          <div v-if="!favorites.length" class="sidebar-empty">
            <FolderOpen :size="28" :stroke-width="1.3" /><strong>{{
              search ? "没有匹配的请求" : "让好用的请求触手可及"
            }}</strong>
            <p>
              {{
                search
                  ? "试试其他关键词"
                  : "保存你的第一个请求，\n下次调试时从这里开始。"
              }}
            </p>
          </div>
        </template>
        <template v-else
          ><button
            v-for="item in history"
            :key="item.id"
            class="history-item"
            :disabled="store.busy"
            @click="store.restore(item.request)"
          >
            <div>
              <span
                class="method-label"
                :class="item.request.method.toLowerCase()"
                >{{ item.request.method }}</span
              ><span>{{ label(item.request.url) }}</span
              ><i :class="{ fail: !item.status || item.status >= 400 }">{{
                item.status || "ERR"
              }}</i>
            </div>
            <small>{{ new Date(item.at).toLocaleString("zh-CN") }}</small>
          </button>
          <div v-if="!history.length" class="sidebar-empty">
            <HistoryIcon :size="28" /><strong>还没有请求记录</strong>
            <p>发送的请求会自动出现在这里。</p>
          </div></template
        >
      </div>
      <div class="sidebar-bottom">
        <div class="local-status">
          <span></span> 数据仅保存在本机 <Globe2 :size="13" />
        </div>
        <div class="sidebar-footer">
          <span>Postdata v0.1.0</span><span>为专注调试而生</span>
        </div>
      </div>
    </aside>
    <main class="main-area">
      <header class="topbar">
        <div class="breadcrumb"><span>请求工作台</span></div>
        <button
          class="quiet-button"
          :disabled="store.busy"
          @click="showCookies"
        >
          <Cookie :size="15" /> Cookie 管理
        </button>
      </header>
      <div class="request-titlebar">
        <div>
          <div class="eyebrow">HTTP REQUEST</div>
          <h1>
            {{ store.request.name || "未命名请求"
            }}<span
              class="unsaved-dot"
              :title="store.favoriteId ? '编辑后请保存更新' : '尚未收藏'"
            ></span>
          </h1>
        </div>
        <div class="request-actions">
          <button
            class="outline-button"
            :disabled="store.busy"
            @click="showCurl"
          >
            <Terminal :size="15" /> 生成 cURL
          </button>
          <button
            class="outline-button"
            :disabled="store.busy"
            @click="openSave"
          >
            <Star :size="15" /> {{ store.favoriteId ? "保存更改" : "保存请求" }}
          </button>
        </div>
      </div>
      <section class="request-workbench">
        <form class="url-bar" @submit.prevent="store.send">
          <select
            v-model="store.request.method"
            aria-label="请求方法"
            :disabled="store.busy"
            :class="store.request.method.toLowerCase()"
          >
            <option
              v-for="method in [
                'GET',
                'POST',
                'PUT',
                'PATCH',
                'DELETE',
                'HEAD',
                'OPTIONS',
              ]"
              :key="method"
            >
              {{ method }}
            </option>
          </select>
          <div class="url-divider"></div>
          <input
            :value="store.request.url"
            :disabled="store.busy"
            placeholder="输入请求 URL，例如 https://httpbin.org/get"
            aria-label="请求 URL"
            spellcheck="false"
            @input="store.setUrl(($event.target as HTMLInputElement).value)"
          /><button v-if="!store.busy" class="send-button" type="submit">
            发送 <Send :size="15" /></button
          ><button
            v-else
            class="send-button cancel-button"
            type="button"
            @click="store.cancel"
          >
            取消 <Square :size="13" />
          </button>
        </form>
        <div class="request-tabs">
          <button
            v-for="item in tabs"
            :key="item.id"
            :class="{ active: tab === item.id }"
            @click="tab = item.id"
          >
            {{ item.label
            }}<span v-if="item.count">{{ item.count }}</span></button
          ><span class="shortcut-hint"><Command :size="11" /> ↵ 发送请求</span>
        </div>
        <div
          class="request-panel"
          :class="{
            'request-panel-code':
              tab === 'body' &&
              ['json', 'text'].includes(store.request.body.kind),
          }"
        >
          <template v-if="tab === 'params'"
            ><div class="section-description">
              <strong>Query 参数</strong><span>自动与 URL 同步</span>
            </div>
            <PairEditor
              :rows="store.request.params"
              :disabled="store.busy"
              @change="store.syncParams"
          /></template>
          <template v-if="tab === 'headers'"
            ><div class="section-description">
              <strong>请求头</strong><span>支持重复键与自定义 Header</span>
            </div>
            <PairEditor :rows="store.request.headers" :disabled="store.busy" />
            <p v-if="store.request.auth.kind !== 'none'" class="inline-note">
              鉴权配置会覆盖手动设置的 Authorization。
            </p></template
          >
          <template v-if="tab === 'body'"
            ><div class="body-types">
              <label
                v-for="[value, title] in [
                  ['none', '无'],
                  ['json', 'JSON'],
                  ['text', '文本'],
                  ['form', 'x-www-form-urlencoded'],
                  ['multipart', 'form-data'],
                ]"
                :key="value"
                ><input
                  v-model="store.request.body.kind"
                  type="radio"
                  :value="value"
                  :disabled="store.busy"
                />{{ title }}</label
              >
            </div>
            <div v-if="store.request.body.kind === 'none'" class="small-empty">
              <Braces :size="26" />
              <p>此请求没有请求体</p>
            </div>
            <CodeEditor
              v-else-if="['json', 'text'].includes(store.request.body.kind)"
              :key="store.request.body.kind"
              v-model="store.request.body.text"
              :readonly="store.busy"
              :is-json="store.request.body.kind === 'json'"
            /><template v-else
              ><PairEditor
                :rows="store.request.body.fields"
                :files="store.request.body.kind === 'multipart'"
                :disabled="store.busy"
                @error="store.error = $event"
              />
              <p
                v-if="store.request.body.kind === 'multipart'"
                class="inline-note"
              >
                Content-Type 与 boundary 自动生成，文件内容仅在发送时读取。
              </p></template
            ></template
          >
          <div v-if="tab === 'auth'" class="settings-panel">
            <label
              >鉴权方式<select
                v-model="store.request.auth.kind"
                :disabled="store.busy"
              >
                <option value="none">无鉴权</option>
                <option value="bearer">Bearer Token</option>
                <option value="basic">Basic Auth</option>
              </select></label
            ><label v-if="store.request.auth.kind === 'bearer'"
              >Token<input
                v-model="store.request.auth.token"
                type="password"
                :disabled="store.busy"
                placeholder="输入访问令牌" /></label
            ><template v-if="store.request.auth.kind === 'basic'"
              ><label
                >用户名<input
                  v-model="store.request.auth.username"
                  :disabled="store.busy" /></label
              ><label
                >密码<input
                  v-model="store.request.auth.password"
                  type="password"
                  :disabled="store.busy" /></label
            ></template>
            <p class="inline-note">
              {{
                store.request.auth.kind === "none"
                  ? "你也可以在请求头中手动填写鉴权信息。"
                  : "此配置将覆盖请求头中的 Authorization。凭据随请求保存在本机。"
              }}
            </p>
          </div>
          <div v-if="tab === 'settings'" class="settings-panel">
            <label
              >请求超时（毫秒）<input
                v-model.number="store.request.timeoutMs"
                type="number"
                min="1"
                max="600000"
                :disabled="store.busy"
            /></label>
            <p class="inline-note">
              默认 30 秒 · 自动跟随最多 10 次重定向 · 验证 TLS 证书<br />响应读取上限
              10 MiB · Cookie 自动保持会话
            </p>
          </div>
        </div>
      </section>
      <div v-if="store.error" class="message error-message" role="alert">
        <span>{{ store.error }}</span
        ><button class="icon-button" title="关闭错误" @click="store.error = ''">
          <X :size="14" />
        </button>
      </div>
      <div v-if="store.notice" class="message notice-message" role="status">
        <span>{{ store.notice }}</span
        ><button
          class="icon-button"
          title="关闭提示"
          @click="store.notice = ''"
        >
          <X :size="14" />
        </button>
      </div>
      <section class="response-section">
        <div class="response-heading">
          <div>
            <ArrowDown :size="15" /><strong>响应</strong
            ><span v-if="!store.response" class="response-label">RESPONSE</span>
          </div>
          <div v-if="store.response" class="response-metrics">
            <span
              class="status-pill"
              :class="{ 'status-error': store.response.status >= 400 }"
              ><i></i>{{ store.response.status }}</span
            ><span><Clock3 :size="12" /> {{ store.response.elapsedMs }} ms</span
            ><span
              >{{ size(store.response.size)
              }}{{ store.response.truncated ? "+" : "" }}</span
            >
          </div>
          <span v-else class="waiting-status"
            ><i :class="{ working: store.busy }"></i
            >{{ store.busy ? "请求进行中" : "等待发送" }}</span
          >
        </div>
        <template v-if="store.response"
          ><div class="response-toolbar">
            <button
              :class="{ active: responseTab === 'body' }"
              @click="responseTab = 'body'"
            >
              响应体</button
            ><button
              :class="{ active: responseTab === 'headers' }"
              @click="responseTab = 'headers'"
            >
              响应头 <span>{{ store.response.headers.length }}</span>
            </button>
            <div
              class="response-tools"
              v-if="responseTab === 'body' && !store.response.binary"
            >
              <button :class="{ selected: pretty }" @click="pretty = true">
                格式化</button
              ><button :class="{ selected: !pretty }" @click="pretty = false">
                原文</button
              ><button
                class="icon-button"
                title="复制响应"
                @click="copyResponse"
              >
                <Check v-if="copied" :size="14" /><Copy v-else :size="14" />
              </button>
            </div>
          </div>
          <div v-if="store.response.truncated" class="limit-notice">
            响应超过 10 MiB，已停止读取；当前仅显示已读取的部分内容。
          </div>
          <div class="final-url" :title="store.response.finalUrl">
            {{ store.response.finalUrl }}
          </div>
          <div v-if="responseTab === 'headers'" class="response-headers">
            <div v-for="([key, value], i) in store.response.headers" :key="i">
              <strong>{{ key }}</strong
              ><span>{{ value }}</span>
            </div>
          </div>
          <div v-else-if="store.response.binary" class="small-empty">
            <FolderOpen :size="28" />
            <p>二进制响应 · {{ size(store.response.size) }}</p>
            <small>{{
              store.response.headers.find(
                ([key]) => key === "content-type",
              )?.[1] || "未知类型"
            }}</small>
          </div>
          <CodeEditor
            v-else
            :model-value="responseText"
            readonly
            :is-json="jsonResponse"
        /></template>
        <div v-else class="response-empty">
          <div class="empty-illustration" :class="{ loading: store.busy }">
            <div class="orbit orbit-one"></div>
            <div class="orbit orbit-two"></div>
            <div class="illustration-card">
              <Send :size="30" :stroke-width="1.4" />
            </div>
            <span class="spark spark-one">+</span
            ><span class="spark spark-two">+</span
            ><span class="spark-dot"></span>
          </div>
          <h2>
            {{ store.busy ? "正在等待响应…" : "每一次探索，从一个请求开始" }}
          </h2>
          <p>
            {{
              store.busy
                ? "请求由本机发送，你可以随时取消。"
                : "输入接口地址并发送请求，在这里查看响应结果。"
            }}
          </p>
          <div v-if="!store.busy" class="empty-shortcut">
            <kbd>⌘</kbd><kbd>Enter</kbd><span>快速发送请求</span>
          </div>
        </div>
      </section>
      <footer class="statusbar">
        <span
          ><i></i
          >{{ store.desktop ? "本地请求引擎就绪" : "浏览器界面预览" }}</span
        ><span
          >HTTP / HTTPS <span class="statusbar-divider">|</span> Postdata ·
          简单，专注，高效</span
        >
      </footer>
    </main>
    <div v-if="curlOpen" class="modal-backdrop" @click.self="curlOpen = false">
      <div
        class="modal curl-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="curl-title"
      >
        <div class="modal-title">
          <h2 id="curl-title"><Terminal :size="20" /> cURL 请求</h2>
          <button class="icon-button" title="关闭" @click="curlOpen = false">
            <X :size="18" />
          </button>
        </div>
        <p>
          复制到 macOS / Linux 的 bash、zsh
          终端即可执行。命令包含当前鉴权信息与匹配的
          Cookie；上传使用本机文件路径。
        </p>
        <div v-if="curlError" class="message error-message" role="alert">
          {{ curlError }}
        </div>
        <div v-if="curlLoading" class="small-empty">正在生成命令…</div>
        <textarea
          v-else-if="curlText"
          class="curl-code"
          :value="curlText"
          readonly
          aria-label="cURL 命令"
          spellcheck="false"
        ></textarea>
        <div class="modal-actions">
          <button class="outline-button" @click="curlOpen = false">关闭</button
          ><button
            class="primary-button"
            :disabled="!curlText || curlLoading"
            @click="copyCurl"
          >
            <Check v-if="curlCopied" :size="14" /><Copy v-else :size="14" />{{
              curlCopied ? "已复制" : "复制命令"
            }}
          </button>
        </div>
      </div>
    </div>
    <div v-if="saveOpen" class="modal-backdrop" @click.self="saveOpen = false">
      <form class="modal save-modal" @submit.prevent="save()">
        <div class="modal-title">
          <h2>保存请求</h2>
          <button
            type="button"
            class="icon-button"
            title="关闭"
            @click="saveOpen = false"
          >
            <X :size="18" />
          </button>
        </div>
        <p>将请求保存到个人收藏，随时继续调试。</p>
        <label
          >请求名称<input
            v-model="saveName"
            autofocus
            placeholder="例如：获取用户列表"
            required
        /></label>
        <div class="modal-actions">
          <button
            type="button"
            class="outline-button"
            @click="saveOpen = false"
          >
            取消</button
          ><button
            v-if="store.favoriteId"
            type="button"
            class="outline-button"
            @click="save(true)"
          >
            另存为</button
          ><button class="primary-button" type="submit">
            <Save :size="14" />{{ store.favoriteId ? "更新收藏" : "保存请求" }}
          </button>
        </div>
      </form>
    </div>
    <div
      v-if="cookieOpen"
      class="modal-backdrop"
      @click.self="cookieOpen = false"
    >
      <div class="modal cookie-modal">
        <div class="modal-title">
          <h2><Cookie :size="20" /> Cookie 管理</h2>
          <button class="icon-button" title="关闭" @click="cookieOpen = false">
            <X :size="18" />
          </button>
        </div>
        <p>请求自动共享 Cookie，会话 Cookie 在退出应用后清除。</p>
        <div v-if="cookieWarning" class="message error-message">
          {{ cookieWarning }}
        </div>
        <div v-if="!cookies.length" class="small-empty">
          <Cookie :size="32" />
          <p>暂无 Cookie</p>
        </div>
        <div v-else class="cookie-list">
          <div v-for="(c, i) in cookies" :key="i" class="cookie-item">
            <div>
              <strong>{{ c.name }}</strong
              ><span class="cookie-domain">{{ c.domain }}{{ c.path }}</span
              ><small
                >{{ c.secure ? "Secure · " : ""
                }}{{ c.httpOnly ? "HttpOnly · " : ""
                }}{{ cookieExpiry(c) }}</small
              ><code>{{ c.value }}</code>
            </div>
            <button
              class="icon-button"
              title="删除 Cookie"
              @click="deleteCookie(c)"
            >
              <Trash2 :size="15" />
            </button>
          </div>
        </div>
        <div class="modal-actions">
          <span>{{ cookies.length }} 个 Cookie</span
          ><button
            class="outline-button danger"
            :disabled="!cookies.length"
            @click="clearCookies"
          >
            <Trash2 :size="14" />清空 Cookie
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
