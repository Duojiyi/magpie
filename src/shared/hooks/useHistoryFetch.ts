import { useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Dispatch, SetStateAction } from "react";
import type { ClipboardEntry } from "../types";

interface UseHistoryFetchOptions {
  debouncedSearch: string;
  typeFilter: string | null;
  persistentLimitEnabled: boolean;
  persistentLimit: number;
  pageSize: number;
  currentOffset: number;
  historyLength: number;
  setHistory: Dispatch<SetStateAction<ClipboardEntry[]>>;
  setCurrentOffset: Dispatch<SetStateAction<number>>;
  setHasMore: Dispatch<SetStateAction<boolean>>;
  isLoadingMore: boolean;
  hasMore: boolean;
  setIsLoadingMore: Dispatch<SetStateAction<boolean>>;
}

export const useHistoryFetch = ({
  debouncedSearch,
  typeFilter,
  persistentLimitEnabled,
  persistentLimit,
  pageSize,
  currentOffset,
  historyLength,
  setHistory,
  setCurrentOffset,
  setHasMore,
  isLoadingMore,
  hasMore,
  setIsLoadingMore
}: UseHistoryFetchOptions) => {
  const loadingRef = useRef(false);
  const fetchSeqRef = useRef(0);
  const resetSeqRef = useRef<number | null>(null);
  const lastRequestedOffsetRef = useRef<number | null>(null);
  const currentOffsetRef = useRef(currentOffset);
  const historyLengthRef = useRef(historyLength);

  useEffect(() => {
    currentOffsetRef.current = currentOffset;
  }, [currentOffset]);

  useEffect(() => {
    historyLengthRef.current = historyLength;
  }, [historyLength]);
  const fetchHistory = useCallback(
    async (reset = false) => {
      // A page load started while a reset is in flight would bump the sequence, so the
      // reset's result got dropped while the stale page (old offset) was appended: new
      // items stayed missing until the next reset. Skip such page loads; the reset's own
      // result decides what comes next.
      if (!reset && resetSeqRef.current !== null) {
        lastRequestedOffsetRef.current = null;
        return;
      }
      const seq = ++fetchSeqRef.current;
      if (reset) resetSeqRef.current = seq;
      try {
        if (reset) {
          lastRequestedOffsetRef.current = null;
        }

        const baseOffset = reset
          ? 0
          : Math.min(currentOffsetRef.current, historyLengthRef.current);

        let data: ClipboardEntry[] = [];

        const hasSearch = debouncedSearch && debouncedSearch.trim().length > 0;

        if (hasSearch) {
          let term = debouncedSearch;
          let tagOnly = false;
          if (term.startsWith("tag:")) {
            term = term.slice(4);
            tagOnly = true;
          }

          try {
            data = await invoke<ClipboardEntry[]>("search_clipboard_history", {
              searchTerm: term,
              limit: 200,
              tagOnly
            });
          } catch (e) {
            console.error("Search failed, falling back", e);
            data = [];
          }

          if (seq !== fetchSeqRef.current) return;
          // Search results are not paginated; always replace list and stop infinite loading.
          setHistory(data);
          setCurrentOffset(data.length);
          setHasMore(false);
        } else {
          const requestedLimit = pageSize + 1; // Use standard page size for DB limit
          const rawData = await invoke<ClipboardEntry[]>("get_clipboard_history", {
            limit: requestedLimit,
            offset: baseOffset,
            contentType: typeFilter || undefined
          });

          if (seq !== fetchSeqRef.current) return;

          const hasMoreNow = rawData.length > pageSize;
          const data = hasMoreNow ? rawData.slice(0, pageSize) : rawData;

          // The backend pages through the merged list (database rows + session-only items), so
          // the offset advances by everything received. Counting database rows only froze the
          // offset at 0 with persistence off, and the list never loaded past its first page.
          const receivedCount = data.length;

          if (reset) {
            setHistory(data);
            setCurrentOffset(receivedCount);
            setHasMore(hasMoreNow);
          } else {
            let nextItems: ClipboardEntry[] = [];
            setHistory((prev) => {
              const existingIds = new Set(prev.map((item) => item.id));
              nextItems = data.filter((item) => !existingIds.has(item.id) || item.id === 0);

              if (nextItems.length === 0) return prev;
              return [...prev, ...nextItems];
            });

            setCurrentOffset(prev => prev + receivedCount);
            // If we didn't add any NEW items but the backend says there are more,
            // it means the items we got were already in our list (maybe shifted due to sorting).
            // We should keep hasMore true so the user can try to load further.
            setHasMore(hasMoreNow);
          }
        }
      } catch (err) {
        console.error("无法获取历史记录", err);
        setHasMore(false);
      } finally {
        if (resetSeqRef.current === seq) resetSeqRef.current = null;
      }
    },
    [
      debouncedSearch,
      typeFilter,
      pageSize,
      persistentLimit,
      persistentLimitEnabled,
      setCurrentOffset,
      setHasMore,
      setHistory
    ]
  );

  const loadMoreHistory = useCallback(async () => {
    if (loadingRef.current || isLoadingMore || !hasMore) return;
    if (debouncedSearch && debouncedSearch.trim().length > 0) return;

    const effectiveOffset = Math.min(currentOffsetRef.current, historyLengthRef.current);
    if (lastRequestedOffsetRef.current === effectiveOffset) return;
    lastRequestedOffsetRef.current = effectiveOffset;

    loadingRef.current = true;
    setIsLoadingMore(true);
    try {
      await fetchHistory(false);
    } finally {
      loadingRef.current = false;
      setIsLoadingMore(false);
    }
  }, [debouncedSearch, fetchHistory, hasMore, isLoadingMore, setIsLoadingMore]);

  return { fetchHistory, loadMoreHistory };
};

