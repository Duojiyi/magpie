import { useEffect } from "react";
import type { RefObject } from "react";
import type { ClipboardEntry } from "../types";
import type { VirtualClipboardListHandle } from "../../features/clipboard/types";

interface UseScrollToSelectionOptions {
  filteredHistory: ClipboardEntry[];
  selectedIndex: number;
  isKeyboardMode: boolean;
  idPrefix?: string;
  pinnedCount?: number;
  virtualListRef?: RefObject<VirtualClipboardListHandle | null>;
}

export const useScrollToSelection = ({
  filteredHistory,
  selectedIndex,
  isKeyboardMode,
  idPrefix = "clipboard-item-",
  pinnedCount = 0,
  virtualListRef
}: UseScrollToSelectionOptions) => {
  useEffect(() => {
    if (isKeyboardMode && selectedIndex >= 0 && selectedIndex < filteredHistory.length) {
      const item = filteredHistory[selectedIndex];
      const isPinned = selectedIndex < pinnedCount;
      if (isPinned) {
        // Pinned items live in the virtual list's header, outside its item index space, so
        // scrollToItem cannot reach them. Always jumping to the top left pinned items that
        // overflow the viewport unreachable by keyboard; scroll the element itself instead.
        const pinnedEl = document.getElementById(`${idPrefix}${item.id}`);
        if (pinnedEl) {
          pinnedEl.scrollIntoView({ behavior: "auto", block: "nearest" });
        } else {
          virtualListRef?.current?.scrollToTop?.();
        }
        return;
      }

      const targetIndex = selectedIndex - pinnedCount;
      if (virtualListRef?.current && targetIndex >= 0) {
        virtualListRef.current.scrollToItem(targetIndex);
        return;
      }

      const el = document.getElementById(`${idPrefix}${item.id}`);
      if (el) {
        el.scrollIntoView({ behavior: "auto", block: "nearest" });
      }
    }
  }, [filteredHistory, selectedIndex, isKeyboardMode, idPrefix, pinnedCount, virtualListRef]);
};


