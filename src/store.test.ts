import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { newRequest } from "./types";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke, isTauri: () => true }));
import { useWorkspace } from "./store";
beforeEach(() => {
  setActivePinia(createPinia());
  invoke.mockReset();
  invoke.mockResolvedValue({ version: 1, favorites: [], history: [] });
});
describe("请求工作台状态", () => {
  it("恢复收藏只编辑，不发送；保存保留收藏 ID", async () => {
    const store = useWorkspace();
    const request = newRequest();
    request.url = "https://example.com";
    await store.restore(request, true);
    expect(invoke).not.toHaveBeenCalled();
    expect(store.favoriteId).toBe(request.id);
    await store.save("新名称");
    expect(invoke).toHaveBeenCalledWith("save_favorite", {
      request: expect.objectContaining({ id: request.id, name: "新名称" }),
    });
    expect(invoke).not.toHaveBeenCalledWith("send_request", expect.anything());
  });
  it("发送使用快照、禁止重复发送，完成后保存返回状态", async () => {
    const store = useWorkspace();
    store.setUrl("http://localhost/echo");
    let finish!: (value: unknown) => void;
    invoke.mockImplementation((command) =>
      command === "send_request"
        ? new Promise((resolve) => {
            finish = resolve;
          })
        : Promise.resolve({ version: 1, favorites: [], history: [] }),
    );
    const pending = store.send();
    expect(store.busy).toBe(true);
    await store.send();
    expect(
      invoke.mock.calls.filter(([command]) => command === "send_request"),
    ).toHaveLength(1);
    const id = invoke.mock.calls[0][1].request.id;
    await store.cancel();
    expect(invoke).toHaveBeenCalledWith("cancel_request", { id });
    finish({ response: null, error: "请求已取消", storageError: null });
    await pending;
    expect(store.busy).toBe(false);
    expect(store.error).toBe("请求已取消");
  });
  it("恢复文件请求检查文件，展示缺失提示", async () => {
    const store = useWorkspace();
    const request = newRequest();
    request.body.kind = "multipart";
    request.body.fields = [
      {
        id: "file",
        key: "file",
        value: "/missing.txt",
        kind: "file",
        enabled: true,
      },
    ];
    invoke.mockResolvedValue(["/missing.txt"]);
    await store.restore(request);
    expect(invoke).toHaveBeenCalledWith("check_files", {
      paths: ["/missing.txt"],
    });
    expect(store.notice).toContain("/missing.txt");
  });
});
