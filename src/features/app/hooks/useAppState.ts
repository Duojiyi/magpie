import { useState } from "react";
import { DEFAULT_THEME } from "../../../shared/config/themes";
import type { ClipboardEntry, Locale } from "../../../shared/types";
import type {
  AppState,
  CardDensity,
  CloudSyncContentPrefs,
  DefaultAppsMap,
  InstalledAppOption,
  QuickPasteModifier,
  SettingsSubpage
} from "../types";
import { DEFAULT_CLOUD_SYNC_CONTENT_PREFS } from "../types";
import type { AiProfile, AppCleanupPolicy } from "../../settings/types";

const DEFAULT_AI_KEY = import.meta.env.VITE_AI_DEFAULT_API_KEY ?? "";

export const useAppState = (): AppState => {
  const [showSettings, setShowSettings] = useState(false);
  const [settingsSubpage, setSettingsSubpage] = useState<SettingsSubpage>("home");
  const [showTagManager, setShowTagManager] = useState(false);
  const [tagManagerEnabled, setTagManagerEnabled] = useState(true);
  const [collapsedGroups, setCollapsedGroups] = useState<Record<string, boolean>>({
    general: true,
    clipboard: true,
    advanced: true,
    appearance: true,
    sync: true,
    cloud_sync: true,
    ai: true,
    file_transfer: true,
    default_apps: true,
    data: true
  });
  const [history, setHistory] = useState<ClipboardEntry[]>([]);
  const [search, setSearch] = useState("");
  const [isComposing, setIsComposing] = useState(false);
  const [searchIsFocused, setSearchIsFocused] = useState(false);
  const [showTagFilter, setShowTagFilter] = useState(false);
  const [tagInput, setTagInput] = useState("");
  const [showEmojiPanel, setShowEmojiPanel] = useState(false);
  const [emojiFavorites, setEmojiFavorites] = useState<string[]>([]);
  const [aiOptionsOpenId, setAiOptionsOpenId] = useState<number | null>(null);
  const [editingTagsId, setEditingTagsId] = useState<number | null>(null);
  const [revealedIds, setRevealedIds] = useState<Set<number>>(new Set());
  const [autoStart, setAutoStart] = useState(true);
  const [deduplicate, setDeduplicate] = useState(true);
  const [persistent, setPersistent] = useState(true);
  const [persistentLimitEnabled, setPersistentLimitEnabled] = useState(true);
  const [persistentLimit, setPersistentLimit] = useState<number>(1000);
  const [appSettings, setAppSettings] = useState<Record<string, string>>({});
  const [defaultApps, setDefaultApps] = useState<DefaultAppsMap>({});
  const [showAppSelector, setShowAppSelector] = useState<string | null>(null);
  const [chatMode, setChatMode] = useState(false);
  const [installedApps, setInstalledApps] = useState<InstalledAppOption[]>([]);
  const [dataPath, setDataPath] = useState<string>("");
  const [hotkey, setHotkey] = useState<string>("Alt+C");
  const [sequentialHotkey, setSequentialHotkey] = useState<string>("Alt+V");
  const [richPasteHotkey, setRichPasteHotkey] = useState<string>("Alt+Shift+V");
  const [searchHotkey, setSearchHotkey] = useState<string>("Alt+F");
  // 敏感标记快捷键，默认 S（需求 17.3，可自定义覆盖）；InAppOnly，仅由 webview keydown 响应
  const [sensitiveHotkey, setSensitiveHotkey] = useState<string>("S");
  const [quickPasteModifier, setQuickPasteModifier] =
    useState<QuickPasteModifier>("disabled");
  // 数字快捷粘贴（Ctrl+1~9，InAppOnly）开关，默认关闭（需求 16.1）
  const [quickPasteInAppEnabled, setQuickPasteInAppEnabled] = useState(false);
  const [sequentialMode, setSequentialModeState] = useState(false);
  const [isRecording, setIsRecording] = useState(false);
  const [isRecordingSequential, setIsRecordingSequential] = useState(false);
  const [isRecordingRich, setIsRecordingRich] = useState(false);
  const [isRecordingSearch, setIsRecordingSearch] = useState(false);
  // 敏感标记快捷键录制态（需求 17.3）
  const [isRecordingSensitive, setIsRecordingSensitive] = useState(false);
  const [deleteAfterPaste, setDeleteAfterPaste] = useState(false);
  const [moveToTopAfterPaste, setMoveToTopAfterPaste] = useState(true);
  const [privacyProtection, setPrivacyProtection] = useState(true);
  const [privacyProtectionKinds, setPrivacyProtectionKinds] = useState<string[]>([
    "phone",
    "idcard",
    "email",
    "secret"
  ]);
  const [privacyProtectionCustomRules, setPrivacyProtectionCustomRules] = useState<string>("");
  const [sensitiveMaskPrefixVisible, setSensitiveMaskPrefixVisible] = useState(3);
  const [sensitiveMaskSuffixVisible, setSensitiveMaskSuffixVisible] = useState(3);
  const [sensitiveMaskEmailDomain, setSensitiveMaskEmailDomain] = useState(false);
  const [cleanupRules, setCleanupRules] = useState<string>("");
  const [appCleanupPolicies, setAppCleanupPolicies] = useState<AppCleanupPolicy[]>([]);
  const [captureFiles, setCaptureFiles] = useState(true);
  const [captureRichText, setCaptureRichText] = useState(false);
  const [richTextSnapshotPreview, setRichTextSnapshotPreview] = useState(true);
  const [silentStart, setSilentStart] = useState(true);
  const [followMouse, setFollowMouse] = useState(false);
  const [showAppBorder, setShowAppBorder] = useState(false);
  const [winClipboardDisabled, setWinClipboardDisabled] = useState(false);
  const [registryWinVEnabled, setRegistryWinVEnabled] = useState(false);
  const [pasteMethod, setPasteMethod] = useState("simulate");
  // 主题默认值跟随 DEFAULT_THEME 常量（themes.ts，现为 ink）。
  // DEFAULT_THEME 类型已收窄为 ThemeId 字面量联合，此处显式标注为 string，
  // 使其与 app/types.ts 中 theme/setTheme 的 StateSetter<string> 全链路类型一致。
  const [theme, setTheme] = useState<string>(DEFAULT_THEME);
  const [colorMode, setColorMode] = useState("system");
  const [showSourceAppIcon, setShowSourceAppIcon] = useState(true);

  const [compactMode, setCompactMode] = useState(false);
  // 卡片密度三档（V5 / 需求 32），默认标准；与 compactMode 协调（紧凑模式叠加更密集布局）
  const [cardDensity, setCardDensity] = useState<CardDensity>("standard");
  const [clipboardItemFontSize, setClipboardItemFontSize] = useState(13);
  const [clipboardTagFontSize, setClipboardTagFontSize] = useState(10);
  const [emojiPanelEnabled, setEmojiPanelEnabled] = useState(false);
  const [emojiPanelTab, setEmojiPanelTab] = useState<"emoji" | "favorites">("emoji");
  const [showHotkeyHint, setShowHotkeyHint] = useState(false);
  const [showAutoCloseHint, setShowAutoCloseHint] = useState(false);
  const [language, setLanguage] = useState<Locale>("zh");
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const [isWindowPinned, setIsWindowPinned] = useState(false);
  const [showSearchBox, setShowSearchBox] = useState(true);
  const [scrollTopButtonEnabled, setScrollTopButtonEnabled] = useState(true);
  const [arrowKeySelection, setArrowKeySelection] = useState(true);
  const [hideTrayIcon, setHideTrayIcon] = useState(false);
  const [hideDockIcon, setHideDockIcon] = useState(false);
  const [edgeDocking, setEdgeDocking] = useState(false);
  const [checkUpdateOnStartup, setCheckUpdateOnStartup] = useState(true);
  const [customBackground, setCustomBackground] = useState<string>("");
  const [customBackgroundOpacity, setCustomBackgroundOpacity] = useState(45);
  const [surfaceOpacity, setSurfaceOpacity] = useState(50);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isKeyboardMode, setIsKeyboardMode] = useState(false);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [currentOffset, setCurrentOffset] = useState(0);
  const [mqttEnabled, setMqttEnabled] = useState(false);
  const [mqttServer, setMqttServer] = useState("");
  const [mqttPort, setMqttPort] = useState("1883");
  const [mqttUser, setMqttUser] = useState("");
  const [mqttPass, setMqttPass] = useState("");
  const [mqttTopic, setMqttTopic] = useState("");
  const [mqttProtocol, setMqttProtocol] = useState("mqtt://");
  const [mqttWsPath, setMqttWsPath] = useState("/mqtt");
  const [mqttNotificationEnabled, setMqttNotificationEnabled] = useState(true);
  const [cloudSyncEnabled, setCloudSyncEnabled] = useState(false);
  const [cloudSyncAuto, setCloudSyncAuto] = useState(true);
  const [cloudSyncProvider, setCloudSyncProvider] = useState<"http" | "webdav">("webdav");
  const [cloudSyncServer, setCloudSyncServer] = useState("");
  const [cloudSyncApiKey, setCloudSyncApiKey] = useState("");
  const [cloudSyncIntervalSec, setCloudSyncIntervalSec] = useState("120");
  const [cloudSyncSnapshotIntervalMin, setCloudSyncSnapshotIntervalMin] = useState("720");
  const [cloudSyncWebdavUrl, setCloudSyncWebdavUrl] = useState("");
  const [cloudSyncWebdavUsername, setCloudSyncWebdavUsername] = useState("");
  const [cloudSyncWebdavPassword, setCloudSyncWebdavPassword] = useState("");
  const [cloudSyncWebdavBasePath, setCloudSyncWebdavBasePath] = useState("tiez-sync");
  const [cloudSyncContentPrefs, setCloudSyncContentPrefs] =
    useState<CloudSyncContentPrefs>(DEFAULT_CLOUD_SYNC_CONTENT_PREFS);
  const [fileServerEnabled, setFileServerEnabled] = useState(false);
  const [fileServerPort, setFileServerPort] = useState("12345");
  const [localIp, setLocalIp] = useState("");
  const [availableIps, setAvailableIps] = useState<string[]>([]);
  const [actualPort, setActualPort] = useState("");
  const [fileTransferPath, setFileTransferPath] = useState("");
  const [fileTransferAutoOpen, setFileTransferAutoOpen] = useState(false);
  const [fileTransferAutoCopy, setFileTransferAutoCopy] = useState(false);
  const [fileServerAutoClose, setFileServerAutoClose] = useState(false);
  const [soundEnabled, setSoundEnabled] = useState(false);
  const [pasteSoundEnabled, setPasteSoundEnabled] = useState(true);
  const [soundVolume, setSoundVolume] = useState(1.0);
  const [aiEnabled, setAiEnabled] = useState(true);
  const [aiTargetLang, setAiTargetLang] = useState("zh");
  const [aiThinkingBudget, setAiThinkingBudget] = useState("1024");
  const [aiProfiles, setAiProfiles] = useState<AiProfile[]>([
    {
      id: "lc_flash_v1",
      baseUrl: "https://api.longcat.chat/openai/v1",
      apiKey: DEFAULT_AI_KEY,
      model: "LongCat-Flash-Chat",
      enableThinking: false
    },
    {
      id: "lc_think_v1",
      baseUrl: "https://api.longcat.chat/openai/v1",
      apiKey: DEFAULT_AI_KEY,
      model: "LongCat-Flash-Thinking",
      enableThinking: true
    },
    {
      id: "lc_think_2601_v1",
      baseUrl: "https://api.longcat.chat/openai/v1",
      apiKey: DEFAULT_AI_KEY,
      model: "LongCat-Flash-Thinking-2601",
      enableThinking: true
    }
  ]);
  const [aiAssignedProfileTask, setAiAssignedProfileTask] = useState("default");
  const [aiAssignedProfileMouthpiece, setAiAssignedProfileMouthpiece] = useState("default");
  const [aiAssignedProfileTranslate, setAiAssignedProfileTranslate] = useState("default");
  const [processingAiId, setProcessingAiId] = useState<number | null>(null);
  const [typeFilter, setTypeFilter] = useState<string | null>(null);

  return {
    showSettings,
    setShowSettings,
    settingsSubpage,
    setSettingsSubpage,
    showTagManager,
    setShowTagManager,
    tagManagerEnabled,
    setTagManagerEnabled,
    collapsedGroups,
    setCollapsedGroups,
    history,
    setHistory,
    search,
    setSearch,
    isComposing,
    setIsComposing,
    searchIsFocused,
    setSearchIsFocused,
    showTagFilter,
    setShowTagFilter,
    tagInput,
    setTagInput,
    showEmojiPanel,
    setShowEmojiPanel,
    emojiFavorites,
    setEmojiFavorites,
    aiOptionsOpenId,
    setAiOptionsOpenId,
    editingTagsId,
    setEditingTagsId,
    revealedIds,
    setRevealedIds,
    autoStart,
    setAutoStart,
    deduplicate,
    setDeduplicate,
    persistent,
    setPersistent,
    persistentLimitEnabled,
    setPersistentLimitEnabled,
    persistentLimit,
    setPersistentLimit,
    appSettings,
    setAppSettings,
    defaultApps,
    setDefaultApps,
    showAppSelector,
    setShowAppSelector,
    chatMode,
    setChatMode,
    installedApps,
    setInstalledApps,
    dataPath,
    setDataPath,
    hotkey,
    setHotkey,
    sequentialHotkey,
    setSequentialHotkey,
    richPasteHotkey,
    setRichPasteHotkey,
    searchHotkey,
    setSearchHotkey,
    sensitiveHotkey,
    setSensitiveHotkey,
    quickPasteModifier,
    setQuickPasteModifier,
    quickPasteInAppEnabled,
    setQuickPasteInAppEnabled,
    sequentialMode,
    setSequentialModeState,
    isRecording,
    setIsRecording,
    isRecordingSequential,
    setIsRecordingSequential,
    isRecordingRich,
    setIsRecordingRich,
    isRecordingSearch,
    setIsRecordingSearch,
    isRecordingSensitive,
    setIsRecordingSensitive,
    deleteAfterPaste,
    setDeleteAfterPaste,
    moveToTopAfterPaste,
    setMoveToTopAfterPaste,
    privacyProtection,
    setPrivacyProtection,
    privacyProtectionKinds,
    setPrivacyProtectionKinds,
    privacyProtectionCustomRules,
    setPrivacyProtectionCustomRules,
    sensitiveMaskPrefixVisible,
    setSensitiveMaskPrefixVisible,
    sensitiveMaskSuffixVisible,
    setSensitiveMaskSuffixVisible,
    sensitiveMaskEmailDomain,
    setSensitiveMaskEmailDomain,
    cleanupRules,
    setCleanupRules,
    appCleanupPolicies,
    setAppCleanupPolicies,
    captureFiles,
    setCaptureFiles,
    captureRichText,
    setCaptureRichText,
    richTextSnapshotPreview,
    setRichTextSnapshotPreview,
    silentStart,
    setSilentStart,
    followMouse,
    setFollowMouse,
    showAppBorder,
    setShowAppBorder,
    winClipboardDisabled,
    setWinClipboardDisabled,
    registryWinVEnabled,
    setRegistryWinVEnabled,
    pasteMethod,
    setPasteMethod,
    theme,
    setTheme,
    colorMode,
    setColorMode,
    showSourceAppIcon,
    setShowSourceAppIcon,

    compactMode,
    setCompactMode,
    cardDensity,
    setCardDensity,
    clipboardItemFontSize,
    setClipboardItemFontSize,
    clipboardTagFontSize,
    setClipboardTagFontSize,
    emojiPanelEnabled,
    setEmojiPanelEnabled,
    emojiPanelTab,
    setEmojiPanelTab,
    showHotkeyHint,
    setShowHotkeyHint,
    showAutoCloseHint,
    setShowAutoCloseHint,
    language,
    setLanguage,
    settingsLoaded,
    setSettingsLoaded,
    isWindowPinned,
    setIsWindowPinned,
    showSearchBox,
    setShowSearchBox,
    scrollTopButtonEnabled,
    setScrollTopButtonEnabled,
    arrowKeySelection,
    setArrowKeySelection,
    hideTrayIcon,
    setHideTrayIcon,
    hideDockIcon,
    setHideDockIcon,
    edgeDocking,
    setEdgeDocking,
    checkUpdateOnStartup,
    setCheckUpdateOnStartup,

    customBackground,
    setCustomBackground,
    customBackgroundOpacity,
    setCustomBackgroundOpacity,
    surfaceOpacity,
    setSurfaceOpacity,
    selectedIndex,
    setSelectedIndex,
    isKeyboardMode,
    setIsKeyboardMode,
    isLoadingMore,
    setIsLoadingMore,
    hasMore,
    setHasMore,
    currentOffset,
    setCurrentOffset,
    mqttEnabled,
    setMqttEnabled,
    mqttServer,
    setMqttServer,
    mqttPort,
    setMqttPort,
    mqttUser,
    setMqttUser,
    mqttPass,
    setMqttPass,
    mqttTopic,
    setMqttTopic,
    mqttProtocol,
    setMqttProtocol,
    mqttWsPath,
    setMqttWsPath,
    mqttNotificationEnabled,
    setMqttNotificationEnabled,
    cloudSyncEnabled,
    setCloudSyncEnabled,
    cloudSyncAuto,
    setCloudSyncAuto,
    cloudSyncProvider,
    setCloudSyncProvider,
    cloudSyncServer,
    setCloudSyncServer,
    cloudSyncApiKey,
    setCloudSyncApiKey,
    cloudSyncIntervalSec,
    setCloudSyncIntervalSec,
    cloudSyncSnapshotIntervalMin,
    setCloudSyncSnapshotIntervalMin,
    cloudSyncWebdavUrl,
    setCloudSyncWebdavUrl,
    cloudSyncWebdavUsername,
    setCloudSyncWebdavUsername,
    cloudSyncWebdavPassword,
    setCloudSyncWebdavPassword,
    cloudSyncWebdavBasePath,
    setCloudSyncWebdavBasePath,
    cloudSyncContentPrefs,
    setCloudSyncContentPrefs,
    fileServerEnabled,
    setFileServerEnabled,
    fileServerPort,
    setFileServerPort,
    localIp,
    setLocalIp,
    availableIps,
    setAvailableIps,
    actualPort,
    setActualPort,
    fileTransferPath,
    setFileTransferPath,
    fileTransferAutoOpen,
    setFileTransferAutoOpen,
    fileTransferAutoCopy,
    setFileTransferAutoCopy,
    fileServerAutoClose,
    setFileServerAutoClose,
    soundEnabled,
    setSoundEnabled,
    pasteSoundEnabled,
    setPasteSoundEnabled,
    soundVolume,
    setSoundVolume,
    aiEnabled,
    setAiEnabled,
    aiTargetLang,
    setAiTargetLang,
    aiThinkingBudget,
    setAiThinkingBudget,
    aiProfiles,
    setAiProfiles,
    aiAssignedProfileTask,
    setAiAssignedProfileTask,
    aiAssignedProfileMouthpiece,
    setAiAssignedProfileMouthpiece,
    aiAssignedProfileTranslate,
    setAiAssignedProfileTranslate,
    processingAiId,
    setProcessingAiId,
    typeFilter,
    setTypeFilter
  };
};
