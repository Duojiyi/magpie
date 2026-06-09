import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ComponentProps, Dispatch, RefObject, ReactNode, SetStateAction } from "react";
import { motion, Reorder, useDragControls } from "framer-motion";
import type { DragControls } from "framer-motion";
import { ArrowUp, Clipboard, Copy, ExternalLink, FileText, SearchX, Sparkles, Tag, Tags } from "lucide-react";
import FileTransferChatView from "../../file-transfer/components/FileTransferChatView";
import SettingsPanel from "../../settings/components/SettingsPanel";
import TagManager from "../../tag/components/TagManager";
import EmojiPanel from "../../emoji/components/EmojiPanel";
import { VirtualClipboardList } from "../../clipboard/components/VirtualClipboardList";
import type { ClipboardEntry } from "../../../shared/types";
import { isGlassTheme } from "../../../shared/config/themes";
import type { CardDensity } from "../types";
import type { VirtualClipboardListHandle } from "../../clipboard/types";

type SettingsPanelProps = ComponentProps<typeof SettingsPanel>;
type RenderItem = (
  item: ClipboardEntry,
  index: number,
  dragControls?: DragControls,
  disableLayout?: boolean
) => ReactNode;

type CopyToClipboard = (
  id: number,
  content: string,
  contentType: string,
  pasteWithFormat?: boolean,
  isPinned?: boolean,
  tags?: string[]
) => Promise<void>;

interface AppMainContentProps {
  t: (key: string) => string;
  theme: string;
  showSettings: boolean;
  showTagManager: boolean;
  tagManagerEnabled: boolean;
  showEmojiPanel: boolean;
  chatMode: boolean;
  localIp: string;
  actualPort: string;
  settingsPanelProps: SettingsPanelProps;
  emojiFavorites: string[];
  setEmojiFavorites: (val: string[] | ((prev: string[]) => string[])) => void;
  emojiPanelTab: "emoji" | "favorites";
  setEmojiPanelTab: (val: "emoji" | "favorites") => void;
  saveSetting: (key: string, val: string) => void;
  filteredHistory: ClipboardEntry[];
  search: string;
  pinnedItems: ClipboardEntry[];
  unpinnedItems: ClipboardEntry[];
  compactMode: boolean;
  cardDensity: CardDensity;
  selectedIndex: number;
  isKeyboardMode: boolean;
  aiEnabled: boolean;
  processingAiId: number | null;
  virtualListRef: RefObject<VirtualClipboardListHandle | null>;
  handlePinnedReorder: (newOrderIds: number[]) => void;
  renderItemContent: RenderItem;
  copyToClipboard: CopyToClipboard;
  openContent: (item: ClipboardEntry) => void;
  setEditingTagsId: Dispatch<SetStateAction<number | null>>;
  setTagInput: Dispatch<SetStateAction<string>>;
  handleAIAction: (id: number, content: string, actionType: string) => void;
  loadMoreHistory: () => void;
  handleListScroll: (offset: number) => void;
  hasMore: boolean;
  isLoadingMore: boolean;
  showScrollTop: boolean;
  onScrollTop: () => void;
}

const getTypeName = (t: (key: string) => string, type: string) => {
  switch (type) {
    case "code":
      return t("type_code");
    case "link":
    case "url":
      return t("type_url");
    case "file":
      return t("type_file");
    case "image":
      return t("type_image");
    case "video":
      return t("type_video");
    case "rich_text":
      return t("type_rich_text");
    default:
      return t("type_text") || "Text";
  }
};

const SelectionCommandDock = ({
  t,
  item,
  aiEnabled,
  isAIProcessing,
  copyToClipboard,
  openContent,
  setEditingTagsId,
  setTagInput,
  handleAIAction
}: {
  t: (key: string) => string;
  item: ClipboardEntry;
  aiEnabled: boolean;
  isAIProcessing: boolean;
  copyToClipboard: CopyToClipboard;
  openContent: (item: ClipboardEntry) => void;
  setEditingTagsId: Dispatch<SetStateAction<number | null>>;
  setTagInput: Dispatch<SetStateAction<string>>;
  handleAIAction: (id: number, content: string, actionType: string) => void;
}) => {
  const canUseAI = aiEnabled && (item.content_type === "text" || item.content_type === "rich_text");
  const typeName = getTypeName(t, item.content_type);
  const sourceName = item.source_app?.trim() || typeName;

  return (
    <div className="selection-command-dock window-no-drag" role="toolbar" aria-label={typeName}>
      <div className="selection-dock-meta">
        <span className="selection-dock-type" aria-hidden="true">
          <FileText size={16} />
        </span>
        <span className="selection-dock-title" title={`${sourceName} · ${typeName}`}>
          {sourceName} · {typeName}
        </span>
      </div>
      <div className="selection-dock-actions">
        <button
          type="button"
          className="btn-icon selection-dock-button primary"
          onClick={() => {
            copyToClipboard(item.id, item.content, item.content_type, false, item.is_pinned, item.tags || [])
              .catch(console.error);
          }}
          title={t("copy")}
        >
          <Copy size={14} />
          <span>{t("copy")}</span>
        </button>
        {canUseAI && (
          <button
            type="button"
            className={`btn-icon selection-dock-button ${isAIProcessing ? "active" : ""}`}
            disabled={isAIProcessing}
            onClick={() => handleAIAction(item.id, item.content, "task")}
            title={t("ai_task")}
          >
            <Sparkles size={14} />
            <span>{isAIProcessing ? t("ai_processing") : t("ai_task")}</span>
          </button>
        )}
        <button
          type="button"
          className="btn-icon selection-dock-button"
          onClick={() => {
            setTagInput("");
            setEditingTagsId(item.id);
          }}
          title={t("tags")}
        >
          <Tag size={14} />
          <span>{t("tags")}</span>
        </button>
        <button
          type="button"
          className="btn-icon selection-dock-button icon-only"
          onClick={() => openContent(item)}
          title={t("open")}
          aria-label={t("open")}
        >
          <ExternalLink size={14} />
        </button>
      </div>
    </div>
  );
};

const SortableItem = ({
  item,
  index,
  renderItem,
  isFirst,
  compactMode,
  onDragStart,
  onDragEnd
}: {
  item: ClipboardEntry;
  index: number;
  renderItem: RenderItem;
  isFirst?: boolean;
  compactMode: boolean;
  onDragStart?: () => void;
  onDragEnd?: () => void;
}) => {
  const controls = useDragControls();
  return (
    <Reorder.Item
      value={item.id}
      dragListener={false}
      dragControls={controls}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      className={isFirst ? "first-virtual-item" : undefined}
      style={{
        listStyle: "none",
        overflow: "visible",
        paddingTop: isFirst ? "4px" : undefined
      }}
    >
      <div style={{ paddingBottom: compactMode ? "2px" : "4px" }}>
        {renderItem(item, index, controls, true)}
      </div>
    </Reorder.Item>
  );
};

const AppMainContent = ({
  t,
  theme,
  showSettings,
  showTagManager,
  tagManagerEnabled,
  showEmojiPanel,
  chatMode,
  localIp,
  actualPort,
  settingsPanelProps,
  emojiFavorites,
  setEmojiFavorites,
  emojiPanelTab,
  setEmojiPanelTab,
  saveSetting,
  filteredHistory,
  search,
  pinnedItems,
  unpinnedItems,
  compactMode,
  cardDensity,
  selectedIndex,
  isKeyboardMode,
  aiEnabled,
  processingAiId,
  virtualListRef,
  handlePinnedReorder,
  renderItemContent,
  copyToClipboard,
  openContent,
  setEditingTagsId,
  setTagInput,
  handleAIAction,
  loadMoreHistory,
  handleListScroll,
  hasMore,
  isLoadingMore,
  showScrollTop,
  onScrollTop
}: AppMainContentProps) => {
  const [pinnedOrderIds, setPinnedOrderIds] = useState<number[]>(
    () => pinnedItems.map((item) => item.id)
  );
  const pinnedOrderRef = useRef<number[]>(pinnedItems.map((item) => item.id));
  const [isDraggingPinned, setIsDraggingPinned] = useState(false);

  useEffect(() => {
    if (isDraggingPinned) return;
    const next = pinnedItems.map((item) => item.id);
    setPinnedOrderIds(next);
    pinnedOrderRef.current = next;
  }, [pinnedItems, isDraggingPinned]);

  const orderedPinnedItems = useMemo(() => {
    if (pinnedItems.length === 0) return [];
    const map = new Map<number, ClipboardEntry>();
    pinnedItems.forEach((item) => map.set(item.id, item));

    const ordered: ClipboardEntry[] = [];
    const seen = new Set<number>();

    pinnedOrderIds.forEach((id) => {
      const item = map.get(id);
      if (!item) return;
      ordered.push(item);
      seen.add(id);
    });

    pinnedItems.forEach((item) => {
      if (!seen.has(item.id)) {
        ordered.push(item);
      }
    });

    return ordered;
  }, [pinnedItems, pinnedOrderIds]);

  const orderedPinnedIds = useMemo(
    () => orderedPinnedItems.map((item) => item.id),
    [orderedPinnedItems]
  );
  const selectedItem = filteredHistory[selectedIndex] ?? null;
  const showSelectionDock = Boolean(selectedItem) && !compactMode && isGlassTheme(theme);

  const handlePinnedIdsReorder = useCallback((nextIds: number[]) => {
    setPinnedOrderIds(nextIds);
    pinnedOrderRef.current = nextIds;
  }, []);

  const handlePinnedDragStart = useCallback(() => {
    setIsDraggingPinned(true);
  }, []);

  const handlePinnedDragEnd = useCallback(() => {
    setIsDraggingPinned(false);
    const finalIds = pinnedOrderRef.current;
    const currentIds = pinnedItems.map((item) => item.id);
    if (
      finalIds.length === currentIds.length &&
      finalIds.every((id, idx) => id === currentIds[idx])
    ) {
      return;
    }
    handlePinnedReorder(finalIds);
  }, [handlePinnedReorder, pinnedItems]);

  if (showTagManager && tagManagerEnabled) {
    return (
      <motion.div
        initial={{ opacity: 0, x: 20 }}
        animate={{ opacity: 1, x: 0 }}
        style={{ height: "100%" }}
      >
        <TagManager t={t} theme={theme} />
      </motion.div>
    );
  }

  if (showEmojiPanel) {
    return (
      <motion.div
        initial={{ opacity: 0, x: 20 }}
        animate={{ opacity: 1, x: 0 }}
        style={{ height: "100%", overflow: "hidden" }}
      >
        <EmojiPanel
          t={t}
          favorites={emojiFavorites}
          setFavorites={setEmojiFavorites}
          activeTab={emojiPanelTab}
          setActiveTab={setEmojiPanelTab}
          saveSetting={saveSetting}
        />
      </motion.div>
    );
  }

  if (showSettings) {
    if (chatMode) {
      return (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          style={{ height: "100%", overflow: "hidden" }}
        >
          <FileTransferChatView t={t} localIp={localIp} actualPort={actualPort} />
        </motion.div>
      );
    }

    return (
      <motion.div
        initial={{ opacity: 0, x: 20 }}
        animate={{ opacity: 1, x: 0 }}
        className={`settings-view ${settingsPanelProps.settingsSubpage === "advanced" ? "advanced-view-shell" : ""}`}
        style={{
          display: "flex",
          flexDirection: "column",
          gap: settingsPanelProps.settingsSubpage === "advanced" ? "0" : "12px",
          height: "100%",
          maxHeight: "100%",
          width: "100%",
          maxWidth: settingsPanelProps.settingsSubpage === "advanced" ? "none" : undefined
        }}
      >
        <SettingsPanel {...settingsPanelProps} />
      </motion.div>
    );
  }

  if (filteredHistory.length === 0) {
    // 三种空状态分别配中英文文案 + lucide 图标（需求 28.1/28.2/28.3）：
    // 1) 标签筛选下无条目：search 以 "tag:" 开头（来自标签下拉，见 AppHeader）
    // 2) 搜索无结果：有普通搜索词
    // 3) 历史为空（全新）：无任何搜索词
    const isTagFilter = search.startsWith("tag:");
    const isSearching = search.length > 0 && !isTagFilter;

    let Icon = Clipboard;
    let title = t("empty_title");
    let desc = t("empty_desc");

    if (isTagFilter) {
      Icon = Tags;
      title = t("empty_tag_title");
      desc = t("empty_tag_desc");
    } else if (isSearching) {
      Icon = SearchX;
      title = t("no_records");
      desc = t("empty_search_desc");
    }

    return (
      <div className="empty-state">
        <Icon size={40} opacity={0.2} style={{ marginBottom: "12px" }} />
        <p
          style={{
            fontSize: "15px",
            fontWeight: "bold",
            color: "var(--text-primary)",
            marginBottom: "4px"
          }}
        >
          {title}
        </p>
        <p style={{ fontSize: "12px", opacity: 0.6 }}>{desc}</p>
      </div>
    );
  }

  return (
    <>
      {filteredHistory.length > 0 && (
        <div className={`history-list-container${showSelectionDock ? " has-selection-dock" : ""}`}>
          <VirtualClipboardList
            ref={virtualListRef}
            items={unpinnedItems}
            compactMode={compactMode}
            cardDensity={cardDensity}
            selectedIndex={selectedIndex - pinnedItems.length}
            isKeyboardMode={isKeyboardMode}
            header={
              pinnedItems.length > 0 ? (
                <Reorder.Group
                  axis="y"
                  values={orderedPinnedIds}
                  onReorder={handlePinnedIdsReorder}
                  className={isDraggingPinned ? "pinned-reorder dragging" : "pinned-reorder"}
                  style={{ listStyle: "none", padding: 0 }}
                >
                  {orderedPinnedItems.map((item, index) => (
                    <SortableItem
                      key={item.id}
                      item={item}
                      index={index}
                      renderItem={renderItemContent}
                      isFirst={index === 0}
                      compactMode={compactMode}
                      onDragStart={handlePinnedDragStart}
                      onDragEnd={handlePinnedDragEnd}
                    />
                  ))}
                </Reorder.Group>
              ) : null
            }
            footer={
              showSelectionDock ? (
                <div className="selection-command-dock-spacer" aria-hidden="true" />
              ) : null
            }
            renderItem={(item, index, isFirst?: boolean) => {
              const el = renderItemContent(item, pinnedItems.length + index, undefined, true);
              if (isFirst && pinnedItems.length === 0) {
                return (
                  <div className="first-virtual-item" style={{ height: "100%", paddingTop: "4px" }}>
                    {el}
                  </div>
                );
              }
              return el;
            }}
            onLoadMore={loadMoreHistory}
            onScroll={handleListScroll}
            hasMore={hasMore}
            isLoading={isLoadingMore}
          />
          {showScrollTop && (
            <button
              type="button"
              className="btn-icon scroll-top-button"
              onClick={onScrollTop}
              aria-label={t("scroll_to_top")}
              title={t("scroll_to_top")}
            >
              <ArrowUp size={16} />
            </button>
          )}
          {showSelectionDock && selectedItem && (
            <SelectionCommandDock
              t={t}
              item={selectedItem}
              aiEnabled={aiEnabled}
              isAIProcessing={processingAiId === selectedItem.id}
              copyToClipboard={copyToClipboard}
              openContent={openContent}
              setEditingTagsId={setEditingTagsId}
              setTagInput={setTagInput}
              handleAIAction={handleAIAction}
            />
          )}
        </div>
      )}
    </>
  );
};

export default AppMainContent;
