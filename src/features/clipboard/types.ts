import type { MouseEvent, ReactNode } from "react";
import type { DragControls } from "framer-motion";
import type { ClipboardEntry, Locale } from "../../shared/types";
import type { CardDensity } from "../app/types";

export interface QuickPasteHint {
  slot: number;
  combo: string;
}

export interface ClipboardItemProps {
  item: ClipboardEntry;
  isSelected: boolean;
  windowPinned: boolean;
  isSensitiveHidden: boolean;
  isRevealed: boolean;
  isEditingTags: boolean;
  tagInput: string;
  /** Tags used elsewhere in history; shown as quick-pick when editing tags */
  tagSuggestions?: string[];
  theme: string;
  language: Locale;
  t: (key: string) => string;
  isAIProcessing?: boolean;
  aiEnabled?: boolean;
  tagColors?: Record<string, string>;
  aiOptionsOpen?: boolean;
  richTextSnapshotPreview?: boolean;
  showSourceAppIcon?: boolean;
  sensitiveMaskPrefixVisible?: number;
  sensitiveMaskSuffixVisible?: number;
  sensitiveMaskEmailDomain?: boolean;
  quickPasteHint?: QuickPasteHint;

  onSelect: () => void;
  onCopy: (withFormat?: boolean) => void;
  onToggleReveal: (e: MouseEvent) => void;
  onOpen: (e: MouseEvent) => void;
  onTogglePin: (e: MouseEvent) => void;
  onDelete: (e: MouseEvent) => void;
  onToggleTagEditor: (e: MouseEvent) => void;
  onTagInput: (val: string) => void;
  onTagAdd: () => void;
  /** Pick an existing tag from the suggestion list (typically closes editor after add) */
  onTagPick?: (tag: string) => void;
  /** Close tag editor without adding (e.g. Escape) */
  onTagEditCancel?: () => void;
  onTagDelete: (tag: string) => void;
  onAIAction?: (type: string) => void;
  onAIOptionsToggle?: () => void;
  onInputSubmit?: (val: string) => void;
  dragControls?: DragControls;
  id?: string;
  disableLayout?: boolean;
}

export type ClipboardRenderItem = (
  item: ClipboardEntry,
  index: number,
  isFirst: boolean
) => ReactNode;

export interface VirtualClipboardListProps {
  items: ClipboardEntry[];
  renderItem: ClipboardRenderItem;
  onLoadMore?: () => void;
  hasMore: boolean;
  isLoading: boolean;
  selectedIndex: number;
  isKeyboardMode: boolean;
  onScroll?: (offset: number) => void;
  compactMode: boolean;
  /** 卡片密度三档（V5 / 需求 32），切换时强制重算虚拟列表行高 */
  cardDensity: CardDensity;
  header?: ReactNode;
  footer?: ReactNode;
}

export interface VirtualClipboardListHandle {
  scrollToItem: (index: number) => void;
  scrollToTop: () => void;
  resetAfterIndex: (index: number) => void;
}
