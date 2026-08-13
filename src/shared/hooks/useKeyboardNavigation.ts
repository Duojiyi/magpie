import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { RefObject } from "react";
import { matchesHotkey } from "./useHotkeyMatching";
import { isMacPlatform } from "../lib/platform";
import { useWindowVisibility } from "./useWindowVisibility";
import { SENSITIVE_TAG } from "../../features/tag/components/FloatingTagInput";
import { normalizeTag, mergeTagsUnion } from "../lib/clipboardCore";
import { parseQuickPasteDigit, quickPasteTargetIndex } from "../lib/quickPasteIndex";
import { shouldHotkeyTrigger } from "../lib/hotkeyScope";
import type { ClipboardEntry } from "../types";

interface UseKeyboardNavigationOptions {
  filteredHistory: ClipboardEntry[];
  selectedIndex: number;
  setSelectedIndex: (val: number | ((prev: number) => number)) => void;
  isKeyboardMode: boolean;
  setIsKeyboardMode: (val: boolean | ((prev: boolean) => boolean)) => void;
  showSettings: boolean;
  showTagManager: boolean;
  chatMode: boolean;
  editingTagsId: number | null;
  arrowKeySelection: boolean;
  richPasteHotkey: string;
  /** 敏感标记快捷键，支持自定义覆盖默认 `S`（需求 17.3） */
  sensitiveHotkey?: string;
  /** 数字快捷粘贴（Ctrl+1~9，InAppOnly）开关；关闭时不拦截、透传至前台（需求 16.6）。默认开启，由 8.2 接线 */
  quickPasteInAppEnabled?: boolean;
  searchInputRef: RefObject<HTMLInputElement | null>;
  copyToClipboard: (id: number, content: string, contentType: string, pasteWithFormat?: boolean) => Promise<void>;
  setSearch: (val: string) => void;
  /** 复用 update_tags 持久化标签并即时刷新列表（来自 useClipboardActions） */
  handleUpdateTags: (id: number, tags: string[]) => Promise<void> | void;
}

/** 浮动标签输入框的目标状态：anchorId 用于定位，targetIds 为全部待打标签的条目 */
interface FloatingTagState {
  anchorId: number;
  targetIds: number[];
}

export interface KeyboardNavigationApi {
  /** 浮动标签输入框是否可见 */
  floatingTagState: FloatingTagState | null;
  /** 提交一个有效标签（已 trim、非空），对全部选中条目去重后持久化并关闭 */
  submitFloatingTag: (tag: string) => void;
  /** 关闭浮动标签输入框，不创建任何标签 */
  closeFloatingTag: () => void;
}

export const useKeyboardNavigation = ({
  filteredHistory,
  selectedIndex,
  setSelectedIndex,
  isKeyboardMode,
  setIsKeyboardMode,
  showSettings,
  showTagManager,
  chatMode,
  editingTagsId,
  arrowKeySelection,
  richPasteHotkey,
  sensitiveHotkey,
  quickPasteInAppEnabled = true,
  searchInputRef,
  copyToClipboard,
  setSearch,
  handleUpdateTags
}: UseKeyboardNavigationOptions) => {
  const filteredHistoryRef = useRef(filteredHistory);
  const selectedIndexRef = useRef(selectedIndex);
  const isKeyboardModeRef = useRef(isKeyboardMode);
  const isWindowVisibleRef = useWindowVisibility();
  const showSettingsRef = useRef(showSettings);
  const showTagManagerRef = useRef(showTagManager);
  const chatModeRef = useRef(chatMode);
  const editingTagsIdRef = useRef(editingTagsId);
  const arrowKeySelectionRef = useRef(arrowKeySelection);
  const copyToClipboardRef = useRef(copyToClipboard);
  const richPasteHotkeyRef = useRef(richPasteHotkey);
  const sensitiveHotkeyRef = useRef(sensitiveHotkey);
  const quickPasteInAppEnabledRef = useRef(quickPasteInAppEnabled);
  const handleUpdateTagsRef = useRef(handleUpdateTags);

  // 浮动标签输入框状态：null 表示未显示（需求 15.1 / 15.5）
  const [floatingTagState, setFloatingTagState] = useState<FloatingTagState | null>(null);
  const floatingTagStateRef = useRef(floatingTagState);
  useEffect(() => { floatingTagStateRef.current = floatingTagState; }, [floatingTagState]);

  useEffect(() => { filteredHistoryRef.current = filteredHistory; }, [filteredHistory]);
  useEffect(() => { selectedIndexRef.current = selectedIndex; }, [selectedIndex]);
  useEffect(() => { isKeyboardModeRef.current = isKeyboardMode; }, [isKeyboardMode]);
  useEffect(() => { showSettingsRef.current = showSettings; }, [showSettings]);
  useEffect(() => { showTagManagerRef.current = showTagManager; }, [showTagManager]);
  useEffect(() => { chatModeRef.current = chatMode; }, [chatMode]);
  useEffect(() => { editingTagsIdRef.current = editingTagsId; }, [editingTagsId]);
  useEffect(() => { arrowKeySelectionRef.current = arrowKeySelection; }, [arrowKeySelection]);
  useEffect(() => { copyToClipboardRef.current = copyToClipboard; }, [copyToClipboard]);
  useEffect(() => { richPasteHotkeyRef.current = richPasteHotkey; }, [richPasteHotkey]);
  useEffect(() => { sensitiveHotkeyRef.current = sensitiveHotkey; }, [sensitiveHotkey]);
  useEffect(() => { quickPasteInAppEnabledRef.current = quickPasteInAppEnabled; }, [quickPasteInAppEnabled]);
  useEffect(() => { handleUpdateTagsRef.current = handleUpdateTags; }, [handleUpdateTags]);
  useEffect(() => {
    invoke("set_navigation_mode", { active: isKeyboardMode }).catch(console.error);
  }, [isKeyboardMode]);

  // 关闭浮动标签输入框，不创建任何标签（需求 15.6）
  const closeFloatingTag = useCallback(() => {
    setFloatingTagState(null);
  }, []);

  // 提交一个有效标签：trim 后对全部选中条目去重持久化（复用 update_tags），保存后关闭并即时刷新（需求 15.2 / 15.4）
  const submitFloatingTag = useCallback((rawTag: string) => {
    const tag = normalizeTag(rawTag);
    const state = floatingTagStateRef.current;
    // 空/纯空白被忽略，输入框保持打开（需求 15.7，由组件侧把关，这里做兜底）
    if (!tag || !state) return;

    const history = filteredHistoryRef.current;
    for (const id of state.targetIds) {
      const item = history.find((entry) => entry.id === id);
      if (!item) continue;
      const current = item.tags || [];
      // 已含同名标签的条目不重复添加（需求 15.2）：并集去重后若无变化则跳过
      const merged = mergeTagsUnion(current, [tag]);
      if (merged.length === current.length) continue;
      void handleUpdateTagsRef.current(id, merged);
    }

    setFloatingTagState(null);
  }, []);

  useEffect(() => {
    let isPastingLocal = false;

    const handleKeyDown = async (e: KeyboardEvent) => {
      if (!isWindowVisibleRef.current) return;
      if (isPastingLocal) {
        e.preventDefault();
        e.stopPropagation();
        return;
      }

      if (
        showSettingsRef.current ||
        showTagManagerRef.current ||
        chatModeRef.current ||
        editingTagsIdRef.current !== null
      ) {
        return;
      }

      const target = e.target as HTMLElement;
      const tagName = target.tagName;
      const isSearchInput = target.classList.contains("search-input");
      const isAnyInput = tagName === "INPUT" || tagName === "TEXTAREA";
      const isEditable = isAnyInput || target.isContentEditable === true;

      if (e.key === "Escape") {
          e.preventDefault();
          if (isEditable) {
              searchInputRef.current?.blur();
          } else {
              const isClipboardAtTop = !isKeyboardModeRef.current || selectedIndexRef.current <= 0;
              if (isClipboardAtTop) {
                invoke("hide_window_cmd");
              } else {
                setIsKeyboardMode(true);
                setSelectedIndex(0);
              }
          }
          return;
      }

      // 快速打标签：选中条目（键盘焦点）按 T 显示浮动标签输入框（需求 15.1）；
      // 未选中任何条目时按 T 不显示、不操作（需求 15.5）。
      if (
        !isEditable &&
        !e.ctrlKey &&
        !e.metaKey &&
        !e.altKey &&
        e.key.toLowerCase() === "t"
      ) {
        const history = filteredHistoryRef.current;
        const index = selectedIndexRef.current;
        const hasSelection = isKeyboardModeRef.current && index >= 0 && index < history.length;
        if (!hasSelection) return;

        e.preventDefault();
        e.stopPropagation();
        if (floatingTagStateRef.current) return;

        const focused = history[index];
        setFloatingTagState({ anchorId: focused.id, targetIds: [focused.id] });
        return;
      }

      // 敏感内容快速标记：选中条目按 S（或自定义快捷键）关联保留标签 __sensitive__（需求 17.1）；
      // 支持自定义快捷键覆盖默认 S（需求 17.3）。
      const customSensitiveHotkey = sensitiveHotkeyRef.current;
      const matchesSensitiveHotkey = customSensitiveHotkey
        ? matchesHotkey(e, customSensitiveHotkey)
        : !isEditable &&
          !e.ctrlKey &&
          !e.metaKey &&
          !e.altKey &&
          !e.shiftKey &&
          e.key.toLowerCase() === "s";
      if (matchesSensitiveHotkey && !isEditable) {
        const history = filteredHistoryRef.current;
        const index = selectedIndexRef.current;
        const hasSelection = isKeyboardModeRef.current && index >= 0 && index < history.length;
        if (!hasSelection) return;

        e.preventDefault();
        e.stopPropagation();

        const focused = history[index];
        const current = focused.tags || [];
        // 已含敏感标签则不重复添加（需求 17.1）
        if (!current.includes(SENSITIVE_TAG)) {
          void handleUpdateTagsRef.current(focused.id, [...current, SENSITIVE_TAG]);
        }
        return;
      }

      // 数字快捷粘贴 Ctrl+1~9（需求 16，scope=InAppOnly）：
      // - 仅主面板可见时由 webview keydown 响应（窗口隐藏在函数顶部已 return → 透传，需求 16.7）；
      // - 不进行全局注册（需求 16.5）；
      // - 开关关闭时不拦截、透传至前台应用（需求 16.6）；
      // - 第 N 个按当前「过滤后可见列表」计；可见条目 < N 时无操作、不报错（需求 16.2/16.3）。
      // 注：成功粘贴后隐藏主面板（需求 16.4）由 8.2 接线。
      if (
        // macOS 上 Cmd+数字 才是原生手势（Ctrl+数字 仍然保留，两者等价）。
        // 这条 webview 路径是 macOS/Linux 唯一的数字快捷粘贴实现——低级键盘钩子是
        // Windows 专属的，而主面板在这两个平台上呼出时会获得焦点，故由 keydown 承接。
        (e.ctrlKey || (isMacPlatform() && e.metaKey)) &&
        !e.altKey &&
        (isMacPlatform() || !e.metaKey) &&
        // 排除 Ctrl+Shift+数字：仅响应纯 Ctrl+1~9，Ctrl+Shift+数字 透传（需求 16.2）
        !e.shiftKey &&
        /^Digit[1-9]$/.test(e.code) &&
        // 开关关闭时透传（需求 16.6）
        quickPasteInAppEnabledRef.current &&
        // Scope=InAppOnly 显式门控：仅主面板可见且 webview 聚焦时触发（需求 16.5/16.7、19.2）。
        // webview keydown 被触发本身即代表 webview 处于聚焦态，故 webviewFocused 传 true。
        shouldHotkeyTrigger("InAppOnly", {
          panelVisible: isWindowVisibleRef.current,
          webviewFocused: true,
        }) &&
        // 仅在非编辑态或搜索框聚焦时响应；浮动标签输入等其他可编辑元素聚焦时透传
        (!isEditable || isSearchInput)
      ) {
        const n = parseQuickPasteDigit(e.code); // Digit1~Digit9 → 1~9
        const history = filteredHistoryRef.current;
        // 第 N 个按过滤后可见列表计；可见条目 < N 时返回 null（需求 16.2/16.3）
        const targetIndex = n === null ? null : quickPasteTargetIndex(n, history.length);

        e.preventDefault();
        e.stopPropagation();

        // 可见条目少于 N：不粘贴、不改状态、不报错（需求 16.3）
        if (targetIndex === null) return;

        const item = history[targetIndex];
        isPastingLocal = true;
        setIsKeyboardMode(false);
        setSelectedIndex(0);

        if (copyToClipboardRef.current) {
          await copyToClipboardRef.current(item.id, item.content, item.content_type, false);
        }

        setTimeout(() => {
          isPastingLocal = false;
        }, 500);
        return;
      }

      if (isEditable) {
          if (e.key === "Enter" && isSearchInput) {
              // Fall through
          } else {
              if (!arrowKeySelectionRef.current) return;
              if (!isSearchInput) return;
          }
      }

      if (arrowKeySelectionRef.current && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
        e.preventDefault();
        e.stopPropagation();

        setIsKeyboardMode((prev) => {
          if (!prev) {
            setSelectedIndex(0);
            return true;
          }

          if (e.key === "ArrowDown") {
            setSelectedIndex((s) => Math.min(s + 1, filteredHistoryRef.current.length - 1));
          } else {
            setSelectedIndex((s) => Math.max(s - 1, 0));
          }
          return true;
        });
        return;
      }

      const matchesRichHotkey = matchesHotkey(e, richPasteHotkeyRef.current);
      const shouldHandleEnter = e.key === "Enter" && isKeyboardModeRef.current;
      if (shouldHandleEnter || matchesRichHotkey) {
        const isRich = matchesRichHotkey;
        e.preventDefault();
        e.stopPropagation();

        const currentIndex = selectedIndexRef.current;
        const currentHistory = filteredHistoryRef.current;

        if (currentIndex >= 0 && currentIndex < currentHistory.length) {
          isPastingLocal = true;
          const item = currentHistory[currentIndex];

          setIsKeyboardMode(false);
          setSelectedIndex(0);

          if (copyToClipboardRef.current) {
            await copyToClipboardRef.current(
              item.id,
              item.content,
              item.content_type,
              isRich
            );
          }

          setTimeout(() => {
            isPastingLocal = false;
          }, 500);
        }
        return;
      }
    };

    const handleInteraction = () => {
      setIsKeyboardMode(false);
    };

    window.addEventListener("keydown", handleKeyDown, true);
    window.addEventListener("mousedown", handleInteraction);

    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      window.removeEventListener("mousedown", handleInteraction);
    };
  }, [searchInputRef, setIsKeyboardMode, setSelectedIndex]);

  useEffect(() => {
    const unlisten = listen<string>("navigation-action", async (event) => {
      try {
        const isVisible = await getCurrentWindow().isVisible();
        if (!isVisible) return;
      } catch (err) {
        console.warn("Failed to check window visibility:", err);
      }

      if (showSettings || showTagManager || chatMode || editingTagsId !== null) return;

      const action = event.payload;
      const history = filteredHistoryRef.current;
      const currentIndex = selectedIndexRef.current;
      const isNavMode = isKeyboardModeRef.current;

      if ((action === "up" || action === "down") && !arrowKeySelection) {
        return;
      }

      if (action === "up") {
        if (!isNavMode) {
          setIsKeyboardMode(true);
          setSelectedIndex(0);
        } else {
          setSelectedIndex((prev) => Math.max(prev - 1, 0));
        }
      } else if (action === "down") {
        if (!isNavMode) {
          setIsKeyboardMode(true);
          setSelectedIndex(0);
        } else {
          setSelectedIndex((prev) => Math.min(prev + 1, history.length - 1));
        }
      } else if (action === "enter") {
        if (!isNavMode) return;
        if (currentIndex >= 0 && currentIndex < history.length) {
          const item = history[currentIndex];
          copyToClipboard(item.id, item.content, item.content_type, false);
        }
      } else if (action === "escape") {
        setSearch("");
        setIsKeyboardMode(false);
      }
    });

    return () => { unlisten.then(f => f()); };
  }, [
    arrowKeySelection,
    chatMode,
    copyToClipboard,
    editingTagsId,
    setIsKeyboardMode,
    setSearch,
    setSelectedIndex,
    showSettings,
    showTagManager
  ]);

  // 选中条目被过滤移除时关闭浮动输入框，避免对不存在的条目打标签
  useEffect(() => {
    if (!floatingTagState) return;
    const stillVisible = floatingTagState.targetIds.some((id) =>
      filteredHistory.some((entry) => entry.id === id)
    );
    if (!stillVisible) setFloatingTagState(null);
  }, [filteredHistory, floatingTagState]);

  const api: KeyboardNavigationApi = {
    floatingTagState,
    submitFloatingTag,
    closeFloatingTag
  };
  return api;
};


