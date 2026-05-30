import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

/**
 * useSettingsInit 启动迁移写回行为单元测试（任务 3.2 / 需求 3.6、3.7、3.8）。
 *
 * 覆盖三条核心逻辑：
 * - 迁移发生（需求 3.6）：归一值≠原始值时，DB（save_setting app.theme）与
 *   localStorage（tiez_theme）各写回恰好一次，且 setTheme 收到归一后的新值。
 * - 未迁移（需求 3.7）：已是新主题值时，不因主题迁移写回 DB，也不写回 tiez_theme。
 * - 写回失败（需求 3.8）：save_setting reject 时不抛错、不中断启动，仍用内存归一值渲染。
 *
 * 测试策略（沿用本仓既有方案，不新增依赖）：
 * - vitest 运行于 node 环境（无 jsdom）。仿照 useHotkeyConfig.test.ts，将 React 的
 *   useState/useEffect/useLayoutEffect mock 为同步直通，使 hook 在普通函数调用下即执行副作用。
 * - 仿照 themes.classes.test.ts 提供最小 DOM 桩（document + classList），供首帧
 *   applyThemeClasses 调用；localStorage 用带 setItem 间谍的桩。
 * - invoke 异步链通过 flush 微任务后断言。
 */

// React：useState 返回初值与空 setter；useEffect/useLayoutEffect 同步执行回调
vi.mock("react", () => ({
  useState: <T>(init: T) => [init, vi.fn()],
  useEffect: (fn: () => void | (() => void)) => {
    fn();
  },
  useLayoutEffect: (fn: () => void | (() => void)) => {
    fn();
  },
}));

// Tauri invoke：按命令分流，get_settings 返回预置 settings，save_setting 记录调用
const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

// Tauri event listen：返回 unlisten promise，测试不依赖事件
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// 运行时判定：强制视为 Tauri 运行时，使初始化副作用生效
vi.mock("../lib/tauriRuntime", () => ({
  isTauriRuntime: () => true,
}));

import { useSettingsInit } from "./useSettingsInit";

// ---------------------------------------------------------------------------
// 最小 DOM / localStorage 桩
// ---------------------------------------------------------------------------
class ClassListStub {
  private readonly tokens = new Set<string>();
  add(...names: string[]): void {
    for (const name of names) this.tokens.add(name);
  }
  remove(...names: string[]): void {
    for (const name of names) this.tokens.delete(name);
  }
  contains(name: string): boolean {
    return this.tokens.has(name);
  }
  [Symbol.iterator](): IterableIterator<string> {
    return this.tokens[Symbol.iterator]();
  }
}

const createElement = (): Element =>
  ({ classList: new ClassListStub() } as unknown as Element);

let setItemSpy: ReturnType<typeof vi.fn>;

/** 安装全局 document 与 localStorage 桩 */
const installGlobals = () => {
  (globalThis as unknown as { document: unknown }).document = {
    documentElement: createElement(),
    body: createElement(),
  };
  setItemSpy = vi.fn();
  (globalThis as unknown as { localStorage: unknown }).localStorage = {
    setItem: setItemSpy,
    getItem: vi.fn(() => null),
    removeItem: vi.fn(),
  };
};

/** 刷新微任务/宏任务队列，等待 invoke().then() 异步链完成 */
const flush = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

/** 取 localStorage 中针对指定 key 的写入调用 */
const writesFor = (key: string) =>
  setItemSpy.mock.calls.filter(([k]) => k === key);

/** 取 invoke 中针对指定命令的调用 */
const invokesOf = (cmd: string) =>
  invokeMock.mock.calls.filter(([c]) => c === cmd);

/** 构造一份完整的 useSettingsInit 入参，集中管理 setter 间谍 */
const buildOptions = () => {
  const setters = {
    setAppSettings: vi.fn(),
    setHotkey: vi.fn(),
    setTheme: vi.fn(),
    setColorMode: vi.fn(),
    setCompactMode: vi.fn(),
    setLanguage: vi.fn(),
  };
  return setters;
};

beforeEach(() => {
  invokeMock.mockReset();
  installGlobals();
});

afterEach(() => {
  delete (globalThis as Record<string, unknown>).document;
  delete (globalThis as Record<string, unknown>).localStorage;
});

describe("useSettingsInit 启动迁移写回（需求 3.6）", () => {
  // 旧值 mica → 归一为 mist：DB 与 localStorage 各写回恰好一次，setTheme 收到归一值
  it("旧值 mica 迁移为 mist：save_setting 与 tiez_theme 各写回一次", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "get_settings") return Promise.resolve({ "app.theme": "mica" });
      return Promise.resolve();
    });
    const setters = buildOptions();

    useSettingsInit(setters);
    await flush();

    // DB 写回恰好一次，键值为归一后的 mist
    expect(invokesOf("save_setting")).toEqual([
      ["save_setting", { key: "app.theme", value: "mist" }],
    ]);
    // localStorage.tiez_theme 写回恰好一次
    expect(writesFor("tiez_theme")).toEqual([["tiez_theme", "mist"]]);
    // 内存渲染使用归一值
    expect(setters.setTheme).toHaveBeenCalledWith("mist");
  });

  // 旧值 acrylic → dusk：同样各写回一次，验证另一条映射
  it("旧值 acrylic 迁移为 dusk：DB 与 localStorage 各写回一次", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "get_settings") return Promise.resolve({ "app.theme": "acrylic" });
      return Promise.resolve();
    });
    const setters = buildOptions();

    useSettingsInit(setters);
    await flush();

    expect(invokesOf("save_setting")).toEqual([
      ["save_setting", { key: "app.theme", value: "dusk" }],
    ]);
    expect(writesFor("tiez_theme")).toEqual([["tiez_theme", "dusk"]]);
    expect(setters.setTheme).toHaveBeenCalledWith("dusk");
  });
});

describe("useSettingsInit 未迁移不写回（需求 3.7）", () => {
  // 已是新值 ink：不因主题迁移写回 DB，也不写回 tiez_theme
  it("已是新值 ink：不写回 app.theme，不写回 tiez_theme", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "get_settings") return Promise.resolve({ "app.theme": "ink" });
      return Promise.resolve();
    });
    const setters = buildOptions();

    useSettingsInit(setters);
    await flush();

    expect(invokesOf("save_setting")).toHaveLength(0);
    expect(writesFor("tiez_theme")).toHaveLength(0);
    expect(setters.setTheme).toHaveBeenCalledWith("ink");
  });

  // 同名保留 paper（LEGACY_THEME_MAP 中 paper→paper，归一值等于原始值）：不写回
  it("同名保留 paper：归一值等于原始值，不触发任何写回", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "get_settings") return Promise.resolve({ "app.theme": "paper" });
      return Promise.resolve();
    });
    const setters = buildOptions();

    useSettingsInit(setters);
    await flush();

    expect(invokesOf("save_setting")).toHaveLength(0);
    expect(writesFor("tiez_theme")).toHaveLength(0);
    expect(setters.setTheme).toHaveBeenCalledWith("paper");
  });
});

describe("useSettingsInit 写回失败容错（需求 3.8）", () => {
  // save_setting reject：不抛错、不中断启动，仍用内存归一值渲染
  it("save_setting 失败时不抛错且 setTheme 仍收到归一值", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "get_settings") return Promise.resolve({ "app.theme": "sakura" });
      if (cmd === "save_setting") return Promise.reject(new Error("db write failed"));
      return Promise.resolve();
    });
    const setters = buildOptions();

    // 调用不应抛出异常
    expect(() => useSettingsInit(setters)).not.toThrow();
    await flush();

    // 即便 DB 写回失败，setTheme 仍使用归一后的内存值（sakura→mist）
    expect(setters.setTheme).toHaveBeenCalledWith("mist");
    // 写回曾被尝试（失败不阻断后续渲染）
    expect(invokesOf("save_setting")).toEqual([
      ["save_setting", { key: "app.theme", value: "mist" }],
    ]);
  });

  // localStorage.setItem 抛错（tiez_theme 写入失败）：被 try/catch 吞掉，不中断启动
  it("localStorage 写入失败时不抛错且 setTheme 仍收到归一值", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "get_settings") return Promise.resolve({ "app.theme": "retro" });
      return Promise.resolve();
    });
    // tiez_theme 写入抛错，其余 key 正常
    setItemSpy.mockImplementation((key: string) => {
      if (key === "tiez_theme") throw new Error("quota exceeded");
    });
    const setters = buildOptions();

    expect(() => useSettingsInit(setters)).not.toThrow();
    await flush();

    // retro→ink，内存归一值仍正常应用
    expect(setters.setTheme).toHaveBeenCalledWith("ink");
  });
});
