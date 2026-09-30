import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Dispatch, SetStateAction } from "react";
import type { ClipboardEntry } from "../types";

interface UsePinnedSortOptions {
  filteredHistory: ClipboardEntry[];
  history: ClipboardEntry[];
  setHistory: Dispatch<SetStateAction<ClipboardEntry[]>>;
}

/**
 * New pinned_order values (higher = first) after the user dragged the visible pinned items
 * into `newOrderIds`. The visible list may be filtered (search / tag): renumbering just the
 * visible items n..1 would collide with hidden pinned items and scramble them, so the visible
 * items' own existing values are handed back out in the new order. If those values are not
 * distinct (e.g. items pinned before ordering existed), every loaded pinned item is renumbered
 * instead, hidden ones keeping their place and the visible slots filled in the new order.
 */
export const computePinnedOrders = (
  history: ClipboardEntry[],
  newOrderIds: number[]
): Map<number, number> => {
  const byId = new Map(history.map((item) => [item.id, item]));
  const existing = newOrderIds.map((id) => byId.get(id)?.pinned_order || 0);
  const orderMap = new Map<number, number>();
  if (new Set(existing).size === existing.length) {
    const values = [...existing].sort((a, b) => b - a);
    newOrderIds.forEach((id, index) => orderMap.set(id, values[index]));
    return orderMap;
  }
  const visible = new Set(newOrderIds);
  const pinned = history
    .filter((item) => item.is_pinned)
    .sort((a, b) => (b.pinned_order || 0) - (a.pinned_order || 0) || b.timestamp - a.timestamp);
  let next = 0;
  const merged = pinned.map((item) => (visible.has(item.id) ? newOrderIds[next++] : item.id));
  // Visible ids missing from history (should not happen) go last.
  merged.push(...newOrderIds.slice(next));
  merged.forEach((id, index) => orderMap.set(id, merged.length - index));
  return orderMap;
};

export const usePinnedSort = ({
  filteredHistory,
  history,
  setHistory
}: UsePinnedSortOptions) => {
  const { pinnedItems, unpinnedItems } = useMemo(() => {
    return {
      pinnedItems: filteredHistory.filter((item) => item.is_pinned),
      unpinnedItems: filteredHistory.filter((item) => !item.is_pinned)
    };
  }, [filteredHistory]);

  const handlePinnedReorder = useCallback(
    async (newOrderIds: number[]) => {
      const orderMap = computePinnedOrders(history, newOrderIds);

      const nextHistory = history.map((item) => {
        const nextOrder = orderMap.get(item.id);
        if (nextOrder !== undefined) {
          return { ...item, pinned_order: nextOrder };
        }
        return item;
      });

      nextHistory.sort((a, b) => {
        if (a.is_pinned !== b.is_pinned) return a.is_pinned ? -1 : 1;
        if (a.is_pinned) {
          if ((a.pinned_order || 0) !== (b.pinned_order || 0)) {
            return (b.pinned_order || 0) - (a.pinned_order || 0);
          }
        }
        return b.timestamp - a.timestamp;
      });

      setHistory(nextHistory);

      const dbOrders = [...orderMap].map(([id, order]) => [id, order]);
      invoke("update_pinned_order", { orders: dbOrders }).catch(console.error);
    },
    [history, setHistory]
  );

  return {
    pinnedItems,
    unpinnedItems,
    handlePinnedReorder
  };
};


