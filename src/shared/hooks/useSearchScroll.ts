import { useCallback, useRef } from "react";
import type { WheelEvent as ReactWheelEvent } from "react";

type UseSearchScrollOptions = {
  /** Persistent "show search box" setting: when on, the box is always visible. */
  showSearchBox: boolean;
  /** Transient reveal (scroll-up at the top / search hotkey). Never persisted. */
  searchRevealed: boolean;
  setSearchRevealed: (val: boolean) => void;
  search: string;
  showSettings: boolean;
  showTagManager: boolean;
};

export const useSearchScroll = ({
  showSearchBox,
  searchRevealed,
  setSearchRevealed,
  search,
  showSettings,
  showTagManager
}: UseSearchScrollOptions) => {
  const scrollTriggerRef = useRef(0);
  const listScrollTopRef = useRef(0);
  const topReachedTimeRef = useRef(0);

  const handleListScroll = useCallback((offset: number) => {
    if (offset === 0 && listScrollTopRef.current > 0) {
      topReachedTimeRef.current = Date.now();
    }
    listScrollTopRef.current = offset;
  }, []);

  const handleMainWheel = useCallback(
    (e: ReactWheelEvent<HTMLElement>) => {
      // Always-visible mode: nothing to reveal or hide (upstream #171).
      if (showSettings || showTagManager || showSearchBox) return;

      if (
        e.deltaY < -5 &&
        (listScrollTopRef.current === 0 || isNaN(listScrollTopRef.current))
      ) {
        if (Date.now() - topReachedTimeRef.current > 250) {
          if (!searchRevealed) {
            scrollTriggerRef.current += Math.abs(e.deltaY);
            if (scrollTriggerRef.current > 45) {
              setSearchRevealed(true);
              scrollTriggerRef.current = 0;
            }
          }
        } else {
          scrollTriggerRef.current = 0;
        }
      } else {
        scrollTriggerRef.current = 0;
      }

      if (e.deltaY > 10 && searchRevealed && search.trim() === "") {
        setSearchRevealed(false);
      }
    },
    [showSettings, showTagManager, showSearchBox, searchRevealed, search, setSearchRevealed]
  );

  return {
    handleListScroll,
    handleMainWheel
  };
};
