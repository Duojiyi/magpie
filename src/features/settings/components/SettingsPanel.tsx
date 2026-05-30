
import { memo, useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, emit } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { ChevronRight, HelpCircle } from "lucide-react";
import { motion } from "framer-motion";
import type { Locale } from "../../../shared/types";
import type { DefaultAppsMap, InstalledAppOption, SettingsSubpage, CloudSyncContentPrefs, CardDensity } from "../../app/types";
import type { AiProfile, AiProfileStatusMap, AppCleanupPolicy, EditableAiProfile } from "../types";
import AppSelectorModal from "./AppSelectorModal";
// Removed UpdateModal imports
import AiProfileModal from "./AiProfileModal";
import GeneralSettingsGroup from "./groups/GeneralSettingsGroup";
import ClipboardSettingsGroup from "./groups/ClipboardSettingsGroup";
import AdvancedSettingsGroup from "./groups/AdvancedSettingsGroup";
import AppearanceSettingsGroup from "./groups/AppearanceSettingsGroup";
import SyncSettingsGroup from "./groups/SyncSettingsGroup";
import CloudSyncSettingsGroup, { type CloudSyncStatusPayload } from "./groups/CloudSyncSettingsGroup";
import DefaultAppsSettingsGroup from "./groups/DefaultAppsSettingsGroup";
import DataSettingsGroup from "./groups/DataSettingsGroup";
import FileTransferSettingsGroup from "./groups/FileTransferSettingsGroup";
import AiSettingsGroup from "./groups/AiSettingsGroup";
import SettingsFooter from "./SettingsFooter";
import { CLOUD_SYNC_ENABLED } from "../../../shared/config/edition";

// 三大分组 tab 标识：常用 / 同步 / 高级
type SettingsTab = "common" | "sync" | "advanced";

// 一次性「设置面板已重组」提示的 localStorage 标记（沿用兼容前缀 tiez_，不可更改）
const SETTINGS_REORG_HINT_KEY = "tiez_settings_reorg_hint_v041";

interface SettingsPanelProps {
    t: (key: string) => string;
    theme: string;
    language: Locale;
    colorMode: string;
    showSourceAppIcon: boolean;
    setShowSourceAppIcon: (val: boolean) => void;
    clipboardItemFontSize: number;
    setClipboardItemFontSize: (val: number) => void;
    clipboardTagFontSize: number;
    setClipboardTagFontSize: (val: number) => void;

    // State
    collapsedGroups: Record<string, boolean>;
    settingsSubpage: SettingsSubpage;
    autoStart: boolean;
    silentStart: boolean;
    persistent: boolean;
    persistentLimitEnabled: boolean;
    persistentLimit: number;
    deduplicate: boolean;
    captureFiles: boolean;
    captureRichText: boolean;
    richTextSnapshotPreview: boolean;
    deleteAfterPaste: boolean;
    moveToTopAfterPaste: boolean;
    sequentialMode: boolean;
    sequentialHotkey: string;
    isRecordingSequential: boolean;
    richPasteHotkey: string;
    isRecordingRich: boolean;
    searchHotkey: string;
    isRecordingSearch: boolean;
    sensitiveHotkey: string;
    isRecordingSensitive: boolean;
    quickPasteModifier: "disabled" | "ctrl" | "alt" | "shift" | "win";
    setQuickPasteModifier: (val: "disabled" | "ctrl" | "alt" | "shift" | "win") => void;
    quickPasteInAppEnabled: boolean;
    setQuickPasteInAppEnabled: (val: boolean) => void;
    privacyProtection: boolean;
    privacyProtectionKinds: string[];
    setPrivacyProtectionKinds: (val: string[]) => void;
    privacyProtectionCustomRules: string;
    setPrivacyProtectionCustomRules: (val: string) => void;
    sensitiveMaskPrefixVisible: number;
    setSensitiveMaskPrefixVisible: (val: number) => void;
    sensitiveMaskSuffixVisible: number;
    setSensitiveMaskSuffixVisible: (val: number) => void;
    sensitiveMaskEmailDomain: boolean;
    setSensitiveMaskEmailDomain: (val: boolean) => void;
    cleanupRules: string;
    setCleanupRules: (val: string) => void;
    appCleanupPolicies: AppCleanupPolicy[];
    setAppCleanupPolicies: (val: AppCleanupPolicy[]) => void;
    hotkey: string;
    showHotkeyHint: boolean;
    showSearchBox: boolean;
    setShowSearchBox: (val: boolean) => void;
    scrollTopButtonEnabled: boolean;
    setScrollTopButtonEnabled: (val: boolean) => void;
    emojiPanelEnabled: boolean;
    setEmojiPanelEnabled: (val: boolean) => void;
    cardDensity: CardDensity;
    setCardDensity: (val: CardDensity) => void;
    tagManagerEnabled: boolean;
    setTagManagerEnabled: (val: boolean) => void;
    arrowKeySelection: boolean;
    setArrowKeySelection: (val: boolean) => void;


    soundEnabled: boolean;
    setSoundEnabled: (val: boolean) => void;
    pasteSoundEnabled: boolean;
    setPasteSoundEnabled: (val: boolean) => void;
    soundVolume: number;
    setSoundVolume: (val: number) => void;
    hideTrayIcon: boolean;
    setHideTrayIcon: (val: boolean) => void;
    hideDockIcon: boolean;
    setHideDockIcon: (val: boolean) => void;
    edgeDocking: boolean;
    setEdgeDocking: (val: boolean) => void;
    checkUpdateOnStartup: boolean;
    setCheckUpdateOnStartup: (val: boolean) => void;
    customBackground: string;
    setCustomBackground: (val: string) => void;
    customBackgroundOpacity: number;
    setCustomBackgroundOpacity: (val: number) => void;
    surfaceOpacity: number;
    setSurfaceOpacity: (val: number) => void;


    mqttEnabled: boolean;
    mqttServer: string;
    mqttPort: string;
    mqttUser: string;
    mqttPass: string;
    mqttTopic: string;
    mqttProtocol: string;
    mqttWsPath: string;
    mqttNotificationEnabled: boolean;
    cloudSyncEnabled: boolean;
    cloudSyncAuto: boolean;
    cloudSyncProvider: "http" | "webdav";
    cloudSyncServer: string;
    cloudSyncApiKey: string;
    cloudSyncIntervalSec: string;
    cloudSyncSnapshotIntervalMin: string;
    cloudSyncWebdavUrl: string;
    cloudSyncWebdavUsername: string;
    cloudSyncWebdavPassword: string;
    cloudSyncWebdavBasePath: string;
    cloudSyncContentPrefs: CloudSyncContentPrefs;
    setCloudSyncContentPrefs: (val: CloudSyncContentPrefs) => void;

    fileServerEnabled: boolean;
    fileServerPort: string;
    localIp: string;
    availableIps?: string[];
    setLocalIp?: (val: string) => void;
    actualPort: string;
    fileTransferAutoOpen: boolean;
    showAutoCloseHint: boolean;
    fileServerAutoClose: boolean;
    fileTransferAutoCopy: boolean;
    fileTransferPath: string;

    installedApps: InstalledAppOption[];
    appSettings: Record<string, string>;
    defaultApps: DefaultAppsMap;
    showAppSelector: string | null;
    dataPath: string;

    // Setters/Actions
    toggleGroup: (group: string) => void;
    setSettingsSubpage: (val: SettingsSubpage) => void;
    setAutoStart: (val: boolean) => void;
    setSilentStart: (val: boolean) => void;
    setPersistent: (val: boolean) => void;
    setPersistentLimitEnabled: (val: boolean) => void;
    setPersistentLimit: (val: number) => void;
    setDeduplicate: (val: boolean) => void;
    setCaptureFiles: (val: boolean) => void;
    setCaptureRichText: (val: boolean) => void;
    setRichTextSnapshotPreview: (val: boolean) => void;
    setDeleteAfterPaste: (val: boolean) => void;
    setMoveToTopAfterPaste: (val: boolean) => void;
    saveAppSetting: (key: string, val: string) => void;
    setSequentialModeState: (val: boolean) => void;
    setIsRecordingSequential: (val: boolean) => void;
    updateSequentialHotkey: (key: string) => void;
    setIsRecordingRich: (val: boolean) => void;
    updateRichPasteHotkey: (key: string) => void;
    setIsRecordingSearch: (val: boolean) => void;
    updateSearchHotkey: (key: string) => void;
    setIsRecordingSensitive: (val: boolean) => void;
    updateSensitiveHotkey: (key: string) => void;
    updateHotkeyScope: (id: 'main' | 'sequential' | 'rich' | 'search', scope: 'Global' | 'InAppOnly' | 'BackgroundOnly') => void;
    resetHotkeyScopes: () => void;
    setPrivacyProtection: (val: boolean) => void;
    setShowHotkeyHint: (val: boolean) => void;
    setIsRecording: (val: boolean) => void;
    isRecording: boolean;
    hotkeyParts: string[];
    updateHotkey: (key: string) => void;

    setTheme: (val: string) => void;
    setColorMode: (val: string) => void;
    setLanguage: (val: Locale) => void;


    compactMode: boolean;
    setCompactMode: (val: boolean) => void;
    checkHotkeyConflict: (newHotkey: string, mode: 'main' | 'sequential' | 'rich' | 'search') => boolean;


    setMqttEnabled: (val: boolean) => void;
    saveMqtt: (key: string, val: string) => void;
    setMqttServer: (val: string) => void;
    setMqttPort: (val: string) => void;
    setMqttUser: (val: string) => void;
    setMqttPass: (val: string) => void;
    setMqttTopic: (val: string) => void;
    setMqttProtocol: (val: string) => void;
    setMqttWsPath: (val: string) => void;
    setMqttNotificationEnabled: (val: boolean) => void;
    setCloudSyncEnabled: (val: boolean) => void;
    setCloudSyncAuto: (val: boolean) => void;
    setCloudSyncProvider: (val: "http" | "webdav") => void;
    setCloudSyncServer: (val: string) => void;
    setCloudSyncApiKey: (val: string) => void;
    setCloudSyncIntervalSec: (val: string) => void;
    setCloudSyncSnapshotIntervalMin: (val: string) => void;
    setCloudSyncWebdavUrl: (val: string) => void;
    setCloudSyncWebdavUsername: (val: string) => void;
    setCloudSyncWebdavPassword: (val: string) => void;
    setCloudSyncWebdavBasePath: (val: string) => void;
    saveCloudSync: (key: string, val: string) => void;

    setFileServerEnabled: (val: boolean) => void;
    setFileServerPort: (val: string) => void;
    setFileTransferAutoOpen: (val: boolean) => void;
    setShowAutoCloseHint: (val: boolean) => void;
    setFileServerAutoClose: (val: boolean) => void;
    setFileTransferAutoCopy: (val: boolean) => void;
    fetchEffectiveTransferPath: () => void;

    setShowAppSelector: (val: string | null) => void;
    handleResetSettings: () => void;
    onOpenChat?: () => void;

    // AI Settings
    aiEnabled: boolean;
    setAiEnabled: (val: boolean) => void;
    aiTargetLang: string;
    setAiTargetLang: (val: string) => void;
    aiThinkingBudget: string;
    setAiThinkingBudget: (val: string) => void;
    saveSetting: (key: string, val: string) => void;
    aiProfiles: AiProfile[];
    setAiProfiles: (val: AiProfile[]) => void;
    aiAssignedProfileTask: string;
    setAiAssignedProfileTask: (id: string) => void;
    aiAssignedProfileMouthpiece: string;
    setAiAssignedProfileMouthpiece: (id: string) => void;
    aiAssignedProfileTranslate: string;
    setAiAssignedProfileTranslate: (id: string) => void;
}

const SettingsPanel = (props: SettingsPanelProps) => {
    const {
        t, theme, language, colorMode, showSourceAppIcon, setShowSourceAppIcon,
        collapsedGroups, settingsSubpage, autoStart, silentStart, persistent, persistentLimitEnabled, persistentLimit, deduplicate, captureFiles, captureRichText, richTextSnapshotPreview, deleteAfterPaste, moveToTopAfterPaste,
        sequentialMode, sequentialHotkey, isRecordingSequential,
        richPasteHotkey, isRecordingRich, searchHotkey, isRecordingSearch, sensitiveHotkey, isRecordingSensitive, quickPasteModifier, setQuickPasteModifier,
        quickPasteInAppEnabled, setQuickPasteInAppEnabled,
        privacyProtection, privacyProtectionKinds, setPrivacyProtectionKinds, privacyProtectionCustomRules, setPrivacyProtectionCustomRules, sensitiveMaskPrefixVisible, setSensitiveMaskPrefixVisible, sensitiveMaskSuffixVisible, setSensitiveMaskSuffixVisible, sensitiveMaskEmailDomain, setSensitiveMaskEmailDomain, cleanupRules, setCleanupRules, appCleanupPolicies, setAppCleanupPolicies, showSearchBox, setShowSearchBox, scrollTopButtonEnabled, setScrollTopButtonEnabled, arrowKeySelection, setArrowKeySelection,
        soundEnabled, setSoundEnabled, pasteSoundEnabled, setPasteSoundEnabled,
        soundVolume, setSoundVolume,
        hideTrayIcon, setHideTrayIcon,
        hideDockIcon, setHideDockIcon,
        edgeDocking, setEdgeDocking,
        checkUpdateOnStartup, setCheckUpdateOnStartup,
        customBackground, setCustomBackground,
        customBackgroundOpacity, setCustomBackgroundOpacity,
        surfaceOpacity, setSurfaceOpacity,
        mqttEnabled, mqttServer, mqttPort, mqttUser, mqttPass, mqttTopic, mqttProtocol, mqttWsPath, mqttNotificationEnabled,
        cloudSyncEnabled, cloudSyncAuto, cloudSyncIntervalSec, cloudSyncSnapshotIntervalMin, cloudSyncWebdavUrl, cloudSyncWebdavUsername, cloudSyncWebdavPassword, cloudSyncWebdavBasePath, cloudSyncContentPrefs,
        fileServerEnabled, fileServerPort, localIp, availableIps, setLocalIp, actualPort, fileTransferAutoOpen, showAutoCloseHint, fileServerAutoClose, fileTransferAutoCopy, fileTransferPath,
        installedApps, appSettings, defaultApps, showAppSelector, dataPath,

        toggleGroup, setSettingsSubpage, setAutoStart, setSilentStart, setPersistent, setPersistentLimitEnabled, setPersistentLimit, setDeduplicate, setCaptureFiles, setCaptureRichText, setRichTextSnapshotPreview, setDeleteAfterPaste, setMoveToTopAfterPaste, saveAppSetting,
        setSequentialModeState, setIsRecordingSequential, updateSequentialHotkey,
        setIsRecordingRich, updateRichPasteHotkey,
        setIsRecordingSearch, updateSearchHotkey,
        setIsRecordingSensitive, updateSensitiveHotkey,
        updateHotkeyScope, resetHotkeyScopes,
        setPrivacyProtection,
        setIsRecording, isRecording, hotkey, hotkeyParts, updateHotkey,
        setTheme, setColorMode, setLanguage, compactMode, setCompactMode, checkHotkeyConflict,
        clipboardItemFontSize, setClipboardItemFontSize, clipboardTagFontSize, setClipboardTagFontSize,
        emojiPanelEnabled, setEmojiPanelEnabled, cardDensity, setCardDensity, tagManagerEnabled, setTagManagerEnabled,
        setMqttEnabled, saveMqtt, setMqttServer, setMqttPort, setMqttUser, setMqttPass, setMqttTopic, setMqttProtocol, setMqttWsPath, setMqttNotificationEnabled,
        setCloudSyncEnabled, setCloudSyncAuto, setCloudSyncIntervalSec, setCloudSyncSnapshotIntervalMin, setCloudSyncWebdavUrl, setCloudSyncWebdavUsername, setCloudSyncWebdavPassword, setCloudSyncWebdavBasePath, setCloudSyncContentPrefs, saveCloudSync,
        setFileServerEnabled, setFileServerPort, setFileTransferAutoOpen, setShowAutoCloseHint, setFileServerAutoClose, setFileTransferAutoCopy, fetchEffectiveTransferPath,
        setShowAppSelector, handleResetSettings,
        aiEnabled, setAiEnabled, aiTargetLang, setAiTargetLang, aiThinkingBudget, setAiThinkingBudget, saveSetting,
        onOpenChat,
        aiProfiles, setAiProfiles, aiAssignedProfileTask, setAiAssignedProfileTask, aiAssignedProfileMouthpiece, setAiAssignedProfileMouthpiece, aiAssignedProfileTranslate, setAiAssignedProfileTranslate
    } = props;

    const [appVersion, setAppVersion] = useState("");
    const [mqttStatus, setMqttStatus] = useState<"connected" | "disconnected" | "connecting">("disconnected");
    const [cloudSyncStatus, setCloudSyncStatus] = useState<CloudSyncStatusPayload>({
        state: "disabled",
        running: false,
        last_sync_at: null,
        last_error: null,
        uploaded_items: 0,
        received_items: 0
    });
    const [cloudSyncNowRunning, setCloudSyncNowRunning] = useState(false);
    const [editingProfile, setEditingProfile] = useState<EditableAiProfile | null>(null);
    const [profileStatuses, setProfileStatuses] = useState<AiProfileStatusMap>({});
    const [updateStatus, setUpdateStatus] = useState<string>("");
    // Removed updateModalData
    const [openHints, setOpenHints] = useState<Set<string>>(new Set());
    const [privacyKindsOpen, setPrivacyKindsOpen] = useState(false);
    const [privacyRulesOpen, setPrivacyRulesOpen] = useState(false);

    // 设置面板三大分组的 tab 切换：常用 / 同步 / 高级
    const [activeTab, setActiveTab] = useState<SettingsTab>("common");

    // 首次升级到重组后版本时，弹一次「设置面板已重组」提示（兼容字段：localStorage 前缀 tiez_）
    useEffect(() => {
        try {
            if (localStorage.getItem(SETTINGS_REORG_HINT_KEY) !== "1") {
                localStorage.setItem(SETTINGS_REORG_HINT_KEY, "1");
                emit("toast", t("settings_reorganized_hint")).catch(console.error);
            }
        } catch (e) {
            console.error("Failed to show settings reorg hint:", e);
        }
    }, [t]);

    const applyFileServerPort = async (portStr: string) => {
        const port = Number(portStr);
        if (!Number.isInteger(port) || port < 1 || port > 65535) return;
        if (!fileServerEnabled) return;
        try {
            await invoke("toggle_file_server", { enabled: false });
            await invoke("toggle_file_server", { enabled: true, port });
        } catch (e) {
            console.error(e);
        }
    };

    const toggleHint = (key: string) => {
        setOpenHints(prev => {
            const next = new Set(prev);
            if (next.has(key)) next.delete(key);
            else next.add(key);
            return next;
        });
    };

    const LabelWithHint = ({ label, hint, hintKey }: { label: string; hint?: string | React.ReactNode; hintKey: string }) => (
        <div className="item-label-group">
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                <span className="item-label">{label}</span>
                {hint && (
                    <button
                        type="button"
                        className="hint-icon-btn"
                        title={typeof hint === 'string' ? hint : undefined}
                        onClick={(e) => {
                            e.stopPropagation();
                            toggleHint(hintKey);
                        }}
                    >
                        <HelpCircle size={12} />
                    </button>
                )}
            </div>
            {hint && openHints.has(hintKey) && (
                typeof hint === 'string' ? <span className="hint">{hint}</span> : hint
            )}
        </div>
    );

    const checkModelStatus = async (profile: AiProfile) => {
        setProfileStatuses(prev => ({ ...prev, [profile.id]: 'loading' }));
        try {
            const result = await invoke<string>("check_ai_connectivity", {
                baseUrl: profile.baseUrl,
                apiKey: profile.apiKey,
                model: profile.model
            });
            if (result === "success") {
                setProfileStatuses(prev => ({ ...prev, [profile.id]: 'success' }));
            }
        } catch (e: unknown) {
            console.error("AI Check failed:", e);
            setProfileStatuses(prev => ({ ...prev, [profile.id]: 'error' }));
        }
    };

    const handleCloudSyncNow = async () => {
        if (!CLOUD_SYNC_ENABLED) return;
        setCloudSyncNowRunning(true);
        try {
            const status = await invoke<CloudSyncStatusPayload>("cloud_sync_now");
            setCloudSyncStatus(status);
        } catch (err) {
            console.error("Cloud sync now failed:", err);
        } finally {
            setCloudSyncNowRunning(false);
        }
    };

    const handleSaveProfile = (profile: EditableAiProfile) => {
        let newProfiles: AiProfile[];
        if (profile.isNew) {
            const { isNew, id: _id, ...rest } = profile;
            newProfiles = [...aiProfiles, { ...rest, id: Date.now().toString() }];
        } else {
            if (!profile.id) return;
            const { isNew, ...rest } = profile;
            const updatedProfile: AiProfile = { ...rest, id: profile.id };
            newProfiles = aiProfiles.map(p => p.id === profile.id ? updatedProfile : p);
        }
        setAiProfiles(newProfiles);
        saveSetting('ai_profiles', JSON.stringify(newProfiles));
        setEditingProfile(null);
    };

    const handleDeleteProfile = (id: string) => {
        // Prevent deleting presets
        if (['lc_flash_v1', 'lc_think_v1', 'lc_think_2601_v1'].includes(id)) return;

        const newProfiles = aiProfiles.filter(p => p.id !== id);
        setAiProfiles(newProfiles);
        saveSetting('ai_profiles', JSON.stringify(newProfiles));
        // Reset assignments if deleted
        if (aiAssignedProfileTask === id) { setAiAssignedProfileTask('default'); saveSetting('ai_assigned_profile_task', 'default'); }
        if (aiAssignedProfileMouthpiece === id) { setAiAssignedProfileMouthpiece('default'); saveSetting('ai_assigned_profile_mouthpiece', 'default'); }
        if (aiAssignedProfileTranslate === id) { setAiAssignedProfileTranslate('default'); saveSetting('ai_assigned_profile_translate', 'default'); }
    };



    useEffect(() => {
        getVersion()
            .then(v => setAppVersion(v))
            .catch(err => {
                console.error("Failed to get version:", err);
                setAppVersion("0.2.0");
            });

        const unlistenMqtt = listen<string>("mqtt-status", (event) => {
            console.log('[MQTT STATUS] Received status:', event.payload);
            setMqttStatus(event.payload as "connected" | "disconnected" | "connecting");
        });

        let unlistenCloud: Promise<() => void> | null = null;
        if (CLOUD_SYNC_ENABLED) {
            unlistenCloud = listen<CloudSyncStatusPayload>("cloud-sync-status", (event) => {
                setCloudSyncStatus(event.payload);
            });
            invoke<CloudSyncStatusPayload>("get_cloud_sync_status")
                .then(setCloudSyncStatus)
                .catch(console.error);
        }

        Promise.all([
            invoke<boolean>("get_mqtt_status"),
            invoke<boolean>("get_mqtt_running")
        ]).then(([connected, running]) => {
            console.log('[MQTT INIT] connected:', connected, 'running:', running);
            if (connected) {
                setMqttStatus("connected");
            } else if (running) {
                setMqttStatus("connecting");
            } else {
                console.log('[MQTT INIT] Keeping default disconnected state');
            }
        }).catch(console.error);

        return () => {
            unlistenMqtt.then(f => f());
            if (unlistenCloud) {
                unlistenCloud.then(f => f());
            }
        };
    }, []);

    const openAdvancedSettingsWindow = useCallback(() => {
        setSettingsSubpage("advanced");
    }, [setSettingsSubpage]);

    return (
        <motion.div
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            style={{ display: 'flex', flexDirection: 'column', gap: '4px', minHeight: '100%', flex: 1 }}
        >
            {settingsSubpage === "advanced" ? (
                <>
                    <AdvancedSettingsGroup
                        t={t}
                        cleanupRules={cleanupRules}
                        setCleanupRules={setCleanupRules}
                        appCleanupPolicies={appCleanupPolicies}
                        setAppCleanupPolicies={setAppCleanupPolicies}
                        installedApps={installedApps}
                    />

                    <AiProfileModal
                        editingProfile={editingProfile}
                        t={t}
                        onClose={() => setEditingProfile(null)}
                        onSave={handleSaveProfile}
                        setEditingProfile={setEditingProfile}
                    />

                    <AppSelectorModal
                        show={showAppSelector}
                        installedApps={installedApps}
                        theme={theme}
                        colorMode={colorMode}
                        t={t}
                        onClose={() => setShowAppSelector(null)}
                        onSave={saveAppSetting}
                    />

                    {/* Removed UpdateModal in advanced */}
                </>
            ) : (
                <>
            {/* 三大分组 tab 切换：常用 / 同步 / 高级 */}
            <div className="settings-tabs" role="tablist">
                <button
                    type="button"
                    role="tab"
                    aria-selected={activeTab === "common"}
                    className={`settings-tab ${activeTab === "common" ? "active" : ""}`}
                    onClick={() => setActiveTab("common")}
                >
                    {t("settings_tab_common")}
                </button>
                <button
                    type="button"
                    role="tab"
                    aria-selected={activeTab === "sync"}
                    className={`settings-tab ${activeTab === "sync" ? "active" : ""}`}
                    onClick={() => setActiveTab("sync")}
                >
                    {t("settings_tab_sync")}
                </button>
                <button
                    type="button"
                    role="tab"
                    aria-selected={activeTab === "advanced"}
                    className={`settings-tab ${activeTab === "advanced" ? "active" : ""}`}
                    onClick={() => setActiveTab("advanced")}
                >
                    {t("settings_tab_advanced")}
                </button>
            </div>

            {/* ===== 常用分组：常规 / 剪贴板 / 界面 ===== */}
            {activeTab === "common" && (
                <>
            {/* General Settings */}
            <GeneralSettingsGroup
                t={t}
                collapsed={collapsedGroups['general']}
                onToggle={() => toggleGroup('general')}
                LabelWithHint={LabelWithHint}
                autoStart={autoStart}
                setAutoStart={setAutoStart}
                silentStart={silentStart}
                setSilentStart={setSilentStart}
                hideTrayIcon={hideTrayIcon}
                setHideTrayIcon={setHideTrayIcon}
                hideDockIcon={hideDockIcon}
                setHideDockIcon={setHideDockIcon}
                edgeDocking={edgeDocking}
                setEdgeDocking={setEdgeDocking}
                checkUpdateOnStartup={checkUpdateOnStartup}
                setCheckUpdateOnStartup={setCheckUpdateOnStartup}
                soundEnabled={soundEnabled}
                setSoundEnabled={setSoundEnabled}
                pasteSoundEnabled={pasteSoundEnabled}
                setPasteSoundEnabled={setPasteSoundEnabled}
                soundVolume={soundVolume}
                setSoundVolume={setSoundVolume}
                showSearchBox={showSearchBox}
                setShowSearchBox={setShowSearchBox}
                scrollTopButtonEnabled={scrollTopButtonEnabled}
                setScrollTopButtonEnabled={setScrollTopButtonEnabled}
                emojiPanelEnabled={emojiPanelEnabled}
                setEmojiPanelEnabled={setEmojiPanelEnabled}
                cardDensity={cardDensity}
                setCardDensity={setCardDensity}
                tagManagerEnabled={tagManagerEnabled}
                setTagManagerEnabled={setTagManagerEnabled}
                arrowKeySelection={arrowKeySelection}
                setArrowKeySelection={setArrowKeySelection}
                saveAppSetting={saveAppSetting}
            />

            {/* Clipboard Settings */}
            <ClipboardSettingsGroup
                t={t}
                collapsed={collapsedGroups['clipboard']}
                onToggle={() => toggleGroup('clipboard')}
                LabelWithHint={LabelWithHint}
                persistent={persistent}
                setPersistent={setPersistent}
                persistentLimitEnabled={persistentLimitEnabled}
                setPersistentLimitEnabled={setPersistentLimitEnabled}
                persistentLimit={persistentLimit}
                setPersistentLimit={setPersistentLimit}
                saveAppSetting={saveAppSetting}
                deduplicate={deduplicate}
                setDeduplicate={setDeduplicate}
                captureFiles={captureFiles}
                setCaptureFiles={setCaptureFiles}
                captureRichText={captureRichText}
                setCaptureRichText={setCaptureRichText}
                richTextSnapshotPreview={richTextSnapshotPreview}
                setRichTextSnapshotPreview={setRichTextSnapshotPreview}
                richPasteHotkey={richPasteHotkey}
                isRecordingRich={isRecordingRich}
                setIsRecordingRich={setIsRecordingRich}
                updateRichPasteHotkey={updateRichPasteHotkey}
                searchHotkey={searchHotkey}
                isRecordingSearch={isRecordingSearch}
                setIsRecordingSearch={setIsRecordingSearch}
                updateSearchHotkey={updateSearchHotkey}
                sensitiveHotkey={sensitiveHotkey}
                isRecordingSensitive={isRecordingSensitive}
                setIsRecordingSensitive={setIsRecordingSensitive}
                updateSensitiveHotkey={updateSensitiveHotkey}
                updateHotkeyScope={updateHotkeyScope}
                resetHotkeyScopes={resetHotkeyScopes}
                quickPasteModifier={quickPasteModifier}
                setQuickPasteModifier={setQuickPasteModifier}
                quickPasteInAppEnabled={quickPasteInAppEnabled}
                setQuickPasteInAppEnabled={setQuickPasteInAppEnabled}
                deleteAfterPaste={deleteAfterPaste}
                setDeleteAfterPaste={setDeleteAfterPaste}
                moveToTopAfterPaste={moveToTopAfterPaste}
                setMoveToTopAfterPaste={setMoveToTopAfterPaste}
                sequentialMode={sequentialMode}
                setSequentialModeState={setSequentialModeState}
                sequentialHotkey={sequentialHotkey}
                isRecordingSequential={isRecordingSequential}
                setIsRecordingSequential={setIsRecordingSequential}
                updateSequentialHotkey={updateSequentialHotkey}
                checkHotkeyConflict={checkHotkeyConflict}
                privacyProtection={privacyProtection}
                setPrivacyProtection={setPrivacyProtection}
                privacyProtectionKinds={privacyProtectionKinds}
                setPrivacyProtectionKinds={setPrivacyProtectionKinds}
                privacyProtectionCustomRules={privacyProtectionCustomRules}
                setPrivacyProtectionCustomRules={setPrivacyProtectionCustomRules}
                sensitiveMaskPrefixVisible={sensitiveMaskPrefixVisible}
                setSensitiveMaskPrefixVisible={setSensitiveMaskPrefixVisible}
                sensitiveMaskSuffixVisible={sensitiveMaskSuffixVisible}
                setSensitiveMaskSuffixVisible={setSensitiveMaskSuffixVisible}
                sensitiveMaskEmailDomain={sensitiveMaskEmailDomain}
                setSensitiveMaskEmailDomain={setSensitiveMaskEmailDomain}
                privacyKindsOpen={privacyKindsOpen}
                setPrivacyKindsOpen={setPrivacyKindsOpen}
                privacyRulesOpen={privacyRulesOpen}
                setPrivacyRulesOpen={setPrivacyRulesOpen}

                isRecording={isRecording}
                setIsRecording={setIsRecording}
                hotkeyParts={hotkeyParts}
                updateHotkey={updateHotkey}
                hotkey={hotkey}
                appSettings={appSettings}
                theme={theme}
                colorMode={colorMode}
            />

            {/* Appearance Settings */}
            <AppearanceSettingsGroup
                t={t}
                collapsed={collapsedGroups['appearance']}
                onToggle={() => toggleGroup('appearance')}
                LabelWithHint={LabelWithHint}
                theme={theme}
                setTheme={setTheme}
                colorMode={colorMode}
                setColorMode={setColorMode}
                language={language}
                setLanguage={setLanguage}
                showSourceAppIcon={showSourceAppIcon}
                setShowSourceAppIcon={setShowSourceAppIcon}
                compactMode={compactMode}
                setCompactMode={setCompactMode}
                clipboardItemFontSize={clipboardItemFontSize}
                setClipboardItemFontSize={setClipboardItemFontSize}
                clipboardTagFontSize={clipboardTagFontSize}
                setClipboardTagFontSize={setClipboardTagFontSize}
                customBackground={customBackground}
                setCustomBackground={setCustomBackground}
                customBackgroundOpacity={customBackgroundOpacity}
                setCustomBackgroundOpacity={setCustomBackgroundOpacity}
                surfaceOpacity={surfaceOpacity}
                setSurfaceOpacity={setSurfaceOpacity}
                saveAppSetting={saveAppSetting}
            />
                </>
            )}

            {/* ===== 同步分组：验证码同步 / 云端同步 / 局域网文件传输 ===== */}
            {activeTab === "sync" && (
                <>
            {/* Sync Settings */}
            <SyncSettingsGroup
                t={t}
                collapsed={collapsedGroups['sync']}
                onToggle={() => toggleGroup('sync')}
                LabelWithHint={LabelWithHint}
                mqttEnabled={mqttEnabled}
                mqttStatus={mqttStatus}
                setMqttEnabled={setMqttEnabled}
                saveMqtt={saveMqtt}
                mqttProtocol={mqttProtocol}
                setMqttProtocol={setMqttProtocol}
                mqttWsPath={mqttWsPath}
                setMqttWsPath={setMqttWsPath}
                mqttServer={mqttServer}
                setMqttServer={setMqttServer}
                mqttPort={mqttPort}
                setMqttPort={setMqttPort}
                mqttUser={mqttUser}
                setMqttUser={setMqttUser}
                mqttPass={mqttPass}
                setMqttPass={setMqttPass}
                mqttTopic={mqttTopic}
                setMqttTopic={setMqttTopic}
                mqttNotificationEnabled={mqttNotificationEnabled}
                setMqttNotificationEnabled={setMqttNotificationEnabled}
            />

            {CLOUD_SYNC_ENABLED && (
                <CloudSyncSettingsGroup
                    t={t}
                    collapsed={collapsedGroups['cloud_sync']}
                    onToggle={() => toggleGroup('cloud_sync')}
                    LabelWithHint={LabelWithHint}
                    cloudSyncEnabled={cloudSyncEnabled}
                    setCloudSyncEnabled={setCloudSyncEnabled}
                    cloudSyncAuto={cloudSyncAuto}
                    setCloudSyncAuto={setCloudSyncAuto}
                    cloudSyncIntervalSec={cloudSyncIntervalSec}
                    setCloudSyncIntervalSec={setCloudSyncIntervalSec}
                    cloudSyncSnapshotIntervalMin={cloudSyncSnapshotIntervalMin}
                    setCloudSyncSnapshotIntervalMin={setCloudSyncSnapshotIntervalMin}
                    cloudSyncWebdavUrl={cloudSyncWebdavUrl}
                    setCloudSyncWebdavUrl={setCloudSyncWebdavUrl}
                    cloudSyncWebdavUsername={cloudSyncWebdavUsername}
                    setCloudSyncWebdavUsername={setCloudSyncWebdavUsername}
                    cloudSyncWebdavPassword={cloudSyncWebdavPassword}
                    setCloudSyncWebdavPassword={setCloudSyncWebdavPassword}
                    cloudSyncWebdavBasePath={cloudSyncWebdavBasePath}
                    setCloudSyncWebdavBasePath={setCloudSyncWebdavBasePath}
                    cloudSyncContentPrefs={cloudSyncContentPrefs}
                    setCloudSyncContentPrefs={setCloudSyncContentPrefs}
                    saveCloudSync={saveCloudSync}
                    status={cloudSyncStatus}
                    syncingNow={cloudSyncNowRunning}
                    onSyncNow={handleCloudSyncNow}
                />
            )}

            {/* File Transfer Settings */}
            <FileTransferSettingsGroup
                t={t}
                collapsed={collapsedGroups['file_transfer']}
                onToggle={() => toggleGroup('file_transfer')}
                fileServerEnabled={fileServerEnabled}
                setFileServerEnabled={setFileServerEnabled}
                fileServerPort={fileServerPort}
                setFileServerPort={setFileServerPort}
                applyFileServerPort={applyFileServerPort}
                localIp={localIp}
                availableIps={availableIps}
                setLocalIp={setLocalIp}
                actualPort={actualPort}
                fileTransferAutoOpen={fileTransferAutoOpen}
                setFileTransferAutoOpen={setFileTransferAutoOpen}
                showAutoCloseHint={showAutoCloseHint}
                setShowAutoCloseHint={setShowAutoCloseHint}
                fileServerAutoClose={fileServerAutoClose}
                setFileServerAutoClose={setFileServerAutoClose}
                fileTransferAutoCopy={fileTransferAutoCopy}
                setFileTransferAutoCopy={setFileTransferAutoCopy}
                onOpenChat={onOpenChat}
                fileTransferPath={fileTransferPath}
                saveSetting={saveSetting}
                fetchEffectiveTransferPath={fetchEffectiveTransferPath}
            />
                </>
            )}

            {/* ===== 高级分组：AI 助手 / 默认程序 / 数据管理 / 高级设置 ===== */}
            {activeTab === "advanced" && (
                <>
            {/* AI Assistant Settings */}
            <AiSettingsGroup
                t={t}
                collapsed={collapsedGroups['ai']}
                onToggle={() => toggleGroup('ai')}
                aiEnabled={aiEnabled}
                setAiEnabled={setAiEnabled}
                saveSetting={saveSetting}
                aiProfiles={aiProfiles}
                profileStatuses={profileStatuses}
                checkModelStatus={checkModelStatus}
                setEditingProfile={setEditingProfile}
                handleDeleteProfile={handleDeleteProfile}
                aiAssignedProfileTask={aiAssignedProfileTask}
                setAiAssignedProfileTask={setAiAssignedProfileTask}
                aiAssignedProfileMouthpiece={aiAssignedProfileMouthpiece}
                setAiAssignedProfileMouthpiece={setAiAssignedProfileMouthpiece}
                aiAssignedProfileTranslate={aiAssignedProfileTranslate}
                setAiAssignedProfileTranslate={setAiAssignedProfileTranslate}
                aiTargetLang={aiTargetLang}
                setAiTargetLang={setAiTargetLang}
                aiThinkingBudget={aiThinkingBudget}
                setAiThinkingBudget={setAiThinkingBudget}
            />

            {/* Default Apps Settings */}
            <DefaultAppsSettingsGroup
                t={t}
                collapsed={collapsedGroups['default_apps']}
                onToggle={() => toggleGroup('default_apps')}
                installedApps={installedApps}
                appSettings={appSettings}
                defaultApps={defaultApps}
                setShowAppSelector={setShowAppSelector}
            />

            {/* Data Management Settings */}
            <DataSettingsGroup
                t={t}
                collapsed={collapsedGroups['data']}
                onToggle={() => toggleGroup('data')}
                dataPath={dataPath}
            />

            <div className="settings-group">
                <button
                    type="button"
                    className="group-header settings-nav-card"
                    onClick={openAdvancedSettingsWindow}
                >
                    <div style={{ minWidth: 0, textAlign: "left" }}>
                        <h3 style={{ margin: 0 }}>{t("advanced_settings")}</h3>
                        <div className="settings-subpage-note">{t("advanced_settings_entry_desc")}</div>
                    </div>
                    <ChevronRight size={16} />
                </button>
            </div>
                </>
            )}

            <SettingsFooter
                t={t}
                appVersion={appVersion}
                updateStatus={updateStatus}
                setUpdateStatus={setUpdateStatus}
                // Removed setUpdateModalData
                onResetSettings={handleResetSettings}
            />

            <AiProfileModal
                editingProfile={editingProfile}
                t={t}
                onClose={() => setEditingProfile(null)}
                onSave={handleSaveProfile}
                setEditingProfile={setEditingProfile}
            />

            <AppSelectorModal
                show={showAppSelector}
                installedApps={installedApps}
                theme={theme}
                colorMode={colorMode}
                t={t}
                onClose={() => setShowAppSelector(null)}
                onSave={saveAppSetting}
            />

            {/* Removed UpdateModal in generic */}
                </>
            )}
        </motion.div>
    );
};

export default memo(SettingsPanel);
