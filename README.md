# Postdata

基于 **Tauri 2 + Vue 3 + TypeScript + Rust / reqwest** 的桌面 HTTP 调试工具。中文浅色工作台，优先支持 macOS。

## 开发

需要 Node.js 22.12+、Rust stable，以及 macOS Xcode Command Line Tools。

```sh
npm ci
npm run tauri dev
```

`npm run dev` 仅启动浏览器界面预览；发送请求、文件选择和保存功能需要在 Tauri 桌面窗口中使用。前端开发端口固定为 1420。

## 使用

- 点击“生成 cURL”可预览并复制当前请求，在 macOS/Linux 的 bash 或 zsh 中执行；无需先发送请求。包含启用的请求头、鉴权、匹配当前 URL 的 Cookie 及请求体。导出命令只保留请求构造参数；代理、重定向、超时和本机配置使用 cURL 默认行为，不继承应用设置。上传命令引用本机文件路径。Cookie 是导出时的快照；跨域重定向和会话更新可能与应用内行为不同。命令不兼容 PowerShell/CMD 的转义规则。
- 选择 HTTP 方法，填写完整 `http://` 或 `https://` URL，点击发送或按 `⌘Enter`（Windows/Linux 为 `Ctrl+Enter`）。
- 拖动左侧栏与主区域之间的竖向分隔线可调整主区域宽度，双击恢复默认；聚焦分隔线后也可用左右方向键调整。
- Query 参数与 URL 双向同步，支持重复键、空值、启停及删除。手动编辑 URL 会重新生成参数表。
- 请求体支持 JSON、文本、URL 编码表单、multipart 文本和文件上传。文件由 Rust 流式读取，不经过前端编码。
- Bearer / Basic 配置覆盖手动 Authorization；multipart 自动生成 Content-Type / boundary。
- 响应支持 JSON 格式化、原文、复制、状态码、耗时、体积、重复响应头及最终 URL。状态码、耗时和体积与格式切换在同一栏显示。拖动请求区与响应区之间的分隔线可调整高度，双击恢复默认；聚焦分隔线后也可用上下方向键调整。HTML 作为文本显示，二进制仅显示类型与体积。
- 默认超时 30 秒，可配置为 1–600000 毫秒；支持取消。最多跟随 10 次重定向，验证 TLS 证书。第一版直接连接，不读取系统或环境变量代理。
- 响应体最多读取 10 MiB，超过时停止读取并显示截断提示；体积代表实际读取的响应体字节数。文本按 UTF-8 显示。
- 收藏可命名、更新、另存为、搜索和删除；历史保留最近 200 次发送快照及结果摘要，恢复后需手动发送。切换请求不会自动保存未收藏的编辑。
- Cookie 自动按域名、路径、Secure 等规则匹配，支持查看、单项删除和清空。会话 Cookie 仅在当前应用进程存活，未过期的持久 Cookie 在重启后恢复。手动 Cookie 请求头由用户负责，优先于自动 Cookie jar。

## 本地数据

应用标识为 `com.postdata.desktop`。macOS 数据目录：

```text
~/Library/Application Support/com.postdata.desktop/
  workspace.json   # 收藏与最近 200 条历史
  cookies.json     # 未过期的持久 Cookie
```

JSON 带版本号，使用临时文件加原子重命名保存；macOS 文件权限为仅当前用户读写。凭据、请求头、请求体及 Cookie 值以明文保存在上述本地文件中，不上传云端。不保存响应正文或上传文件内容，只保存上传路径。历史恢复时会检查文件可读性。

数据损坏或版本不兼容时显示错误并保留原文件，避免覆盖。若需重置，请先退出应用，备份并移走对应文件后重新打开。

## 测试

```sh
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Rust 测试使用自动分配端口的本地 HTTP 服务，覆盖 HTTP 方法、重复参数/请求头、鉴权、请求体、文件上传、取消超时、重定向、非成功状态码、二进制、响应限制、Cookie 规则及持久化。前端测试覆盖参数同步、恢复不发送、收藏 ID、请求快照和取消；cURL 集成测试实际调用本机 curl，验证特殊字符转义、鉴权、重复参数和文件上传。

可手动启动本地验收服务：

```sh
node scripts/test-server.mjs
```

在桌面应用发送 `http://127.0.0.1:4318/echo?a=1&a=2`；`/cookies` 写入测试 Cookie，`/slow` 等待 5 秒便于验证取消。

## 打包

```sh
npm run package:mac
```

该命令使用 Tauri 的 CI 模式生成 `.app` / `.dmg`，跳过 Finder 窗口美化，因此不需要授权 Apple Events。也可用 `npm run tauri build` 执行标准打包。

当前 Mac 架构产物：

- `src-tauri/target/release/bundle/macos/Postdata.app`
- `src-tauri/target/release/bundle/dmg/Postdata_0.1.0_aarch64.dmg`（Apple Silicon）

产物仅供本地使用，未进行开发者证书签名或 Apple 公证。Windows/Linux 需在对应平台安装 Tauri 系统依赖并验证；当前配置的打包目标为 macOS。

## 代码结构

- `src/`：Vue 工作台、Pinia 状态、CodeMirror 编辑器与参数编辑器。
- `src-tauri/src/engine.rs`：HTTP 请求、流式上传、取消、超时及受限响应读取。
- `src-tauri/src/lib.rs`：Tauri 命令、共享 Cookie jar、在途请求与应用数据管理。
- `src-tauri/src/storage.rs`：带版本检查的本地原子持久化。

第一版不包含多标签、环境变量、集合目录、cURL 导入、OAuth、WebSocket、脚本、代理设置、云同步或主题切换。
