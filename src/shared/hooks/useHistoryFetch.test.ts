import { describe, it, expect, vi, beforeEach } from "vitest";

/**
 * useHistoryFetch 的 IPC 参数契约回归测试（审计 P1-1）。
 *
 * 后端 `get_clipboard_history` 命令的 Rust 形参是 `content_type: Option<String>`，
 * Tauri v2 期望前端传 **camelCase**（`contentType`）并自动映射到 snake_case。
 * 曾经这里错传了 snake_case 的 `content_type`，导致按类型筛选在后端被静默忽略、
 * 分页随之失效。本测试锁定该契约：类型筛选必须以 `contentType` 键下发，
 * 且请求参数里绝不能再出现 snake_case 的 `content_type`。
 *
 * 测试策略沿用本仓既有方案（见 useSettingsInit.test.ts）：node 环境下把 React 的
 * useRef/useEffect/useCallback mock 成同步直通，使 hook 可在普通函数调用下执行；
 * invoke 被 mock 后按命令记录调用参数。
 */

vi.mock("react", () => ({
  useRef: <T>(init: T) => ({ current: init }),
  useEffect: (fn: () => void | (() => void)) => {
    fn();
  },
  useCallback: <T>(fn: T) => fn,
}));

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import { useHistoryFetch } from "./useHistoryFetch";

type Options = Parameters<typeof useHistoryFetch>[0];

/** 构造一份完整入参，允许覆盖个别字段；所有 setter 均为间谍 */
const buildOptions = (overrides: Partial<Options> = {}): Options => ({
  debouncedSearch: "",
  typeFilter: null,
  persistentLimitEnabled: false,
  persistentLimit: 0,
  pageSize: 80,
  currentOffset: 0,
  historyLength: 0,
  setHistory: vi.fn(),
  setCurrentOffset: vi.fn(),
  setHasMore: vi.fn(),
  isLoadingMore: false,
  hasMore: false,
  setIsLoadingMore: vi.fn(),
  ...overrides,
});

/** 取 invoke 中针对指定命令的首个调用参数对象 */
const firstArgsOf = (cmd: string): Record<string, unknown> | undefined => {
  const call = invokeMock.mock.calls.find(([c]) => c === cmd);
  return call?.[1] as Record<string, unknown> | undefined;
};

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockResolvedValue([]);
});

describe("useHistoryFetch — get_clipboard_history 参数名契约（P1-1）", () => {
  it("类型筛选以 camelCase 的 contentType 键下发", async () => {
    const { fetchHistory } = useHistoryFetch(buildOptions({ typeFilter: "image" }));
    await fetchHistory(true);

    const args = firstArgsOf("get_clipboard_history");
    expect(args).toBeDefined();
    expect(args).toMatchObject({ contentType: "image" });
  });

  it("请求参数绝不使用 snake_case 的 content_type", async () => {
    const { fetchHistory } = useHistoryFetch(buildOptions({ typeFilter: "image" }));
    await fetchHistory(true);

    const args = firstArgsOf("get_clipboard_history");
    expect(args).toBeDefined();
    expect(Object.keys(args as object)).not.toContain("content_type");
  });

  it("无类型筛选时 contentType 为 undefined（键名仍为 camelCase）", async () => {
    const { fetchHistory } = useHistoryFetch(buildOptions({ typeFilter: null }));
    await fetchHistory(true);

    const args = firstArgsOf("get_clipboard_history");
    expect(args).toBeDefined();
    expect(args).toHaveProperty("contentType", undefined);
    expect(Object.keys(args as object)).not.toContain("content_type");
  });
});
