import { useEffect } from "react";
import type { RefObject } from "react";

/**
 * Give every settings toggle an accessible name. The visible label is a sibling span
 * (`.item-label`) and the wrapping `<label class="switch">` only contains the decorative
 * toggle, so screen readers announced "checkbox, checked" with no name.
 *
 * ponytail: patches the DOM from the row's label text instead of threading an aria-label
 * through ~40 switches; the upgrade path is a shared <Switch label=…> component.
 */
export const applySwitchAccessibleNames = (root: ParentNode) => {
  root
    .querySelectorAll<HTMLInputElement>('.setting-item input[type="checkbox"]')
    .forEach((input) => {
      const label = input
        .closest(".setting-item")
        ?.querySelector(".item-label")
        ?.textContent?.trim();
      if (label && input.getAttribute("aria-label") !== label) {
        input.setAttribute("aria-label", label);
      }
    });
};

export const useSwitchAccessibleNames = (rootRef: RefObject<HTMLElement | null>) => {
  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    applySwitchAccessibleNames(root);
    // Groups expand/collapse and labels change with the language; attribute changes are not
    // observed, so setting aria-label cannot re-trigger the observer.
    const observer = new MutationObserver(() => applySwitchAccessibleNames(root));
    observer.observe(root, { childList: true, subtree: true, characterData: true });
    return () => observer.disconnect();
  }, [rootRef]);
};
