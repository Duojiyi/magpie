import { useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { translations } from "../../../locales";
import AdvancedSettingsGroup from "./groups/AdvancedSettingsGroup";
import { useAppState } from "../../app/hooks/useAppState";
import { useSettingsInit } from "../../../shared/hooks/useSettingsInit";
import { useSettingsPostInit } from "../../../shared/hooks/useSettingsPostInit";
import { useAppBootstrap } from "../../../shared/hooks/useAppBootstrap";
import { useSettingsApply } from "../../../shared/hooks/useSettingsApply";
import { useCustomBackground } from "../../../shared/hooks/useCustomBackground";

const AdvancedSettingsWindow = () => {
    const appState = useAppState();
    const {
        setAppSettings,
        setHotkey,
        setTheme,
        setColorMode,
        setCompactMode,
        language,
        setLanguage,
        customBackground,
        setCustomBackground,
        customBackgroundOpacity,
        setCustomBackgroundOpacity,
        surfaceOpacity,
        setSurfaceOpacity,
        setPersistent,
        setPersistentLimitEnabled,
        setPersistentLimit,
        setDeduplicate,
        setCaptureFiles,
        setCaptureRichText,
        setRichTextSnapshotPreview,
        setPrivacyProtection,
        setPrivacyProtectionKinds,
        setPrivacyProtectionCustomRules,
        cleanupRules,
        setCleanupRules,
        appCleanupPolicies,
        setAppCleanupPolicies,
        setSensitiveMaskPrefixVisible,
        setSensitiveMaskSuffixVisible,
        setSensitiveMaskEmailDomain,
        setSilentStart,
        setFollowMouse,
        showAppBorder,
        setShowAppBorder,
        setShowSourceAppIcon,
        setDeleteAfterPaste,
        setMoveToTopAfterPaste,
        setHideTrayIcon,
        setEdgeDocking,
        setCheckUpdateOnStartup,
        setShowSearchBox,
        setScrollTopButtonEnabled,
        setArrowKeySelection,
        setMqttEnabled,
        setMqttServer,
        setRegistryWinVEnabled,
        setMqttPort,
        setMqttUser,
        setMqttPass,
        setMqttTopic,
        setMqttProtocol,
        setMqttWsPath,
        setMqttNotificationEnabled,
        setCloudSyncEnabled,
        setCloudSyncAuto,
        setCloudSyncProvider,
        setCloudSyncServer,
        setCloudSyncApiKey,
        setCloudSyncIntervalSec,
        setCloudSyncSnapshotIntervalMin,
        setCloudSyncWebdavUrl,
        setCloudSyncWebdavUsername,
        setCloudSyncWebdavPassword,
        setCloudSyncWebdavBasePath,
        setCloudSyncE2eEnabled,
        setCloudSyncE2ePassphrase,
        setFileServerAutoClose,
        setFileTransferAutoOpen,
        setFileTransferAutoCopy,
        setFileServerPort,
        setSequentialHotkey,
        setRichPasteHotkey,
        setSearchHotkey,
        setSensitiveHotkey,
        setQuickPasteModifier,
        setQuickPasteInAppEnabled,
        setSequentialModeState,
        theme,
        colorMode,
        compactMode,
        cardDensity,
        setCardDensity,
        settingsLoaded,
        clipboardItemFontSize,
        setClipboardItemFontSize,
        clipboardTagFontSize,
        setClipboardTagFontSize,
        setEmojiPanelEnabled,
        setTagManagerEnabled,
        setEmojiPanelTab,
        setEmojiFavorites,
        setAutoStart,
        setWinClipboardDisabled,
        setDefaultApps,
        setFileServerEnabled,
        setActualPort,
        setLocalIp,
        setAvailableIps,
        setDataPath,
        setInstalledApps,
        setIsWindowPinned,
        setAiEnabled,
        setAiTargetLang,
        setAiThinkingBudget,
        setAiProfiles,
        setAiAssignedProfileTask,
        setAiAssignedProfileMouthpiece,
        setAiAssignedProfileTranslate,
        setSettingsLoaded,
        setSoundEnabled,
        setSoundVolume,
        setPasteSoundEnabled,
        setPasteMethod,
        installedApps,
        setFileTransferPath,
        hideDockIcon: _hideDockIcon,
        setHideDockIcon,
        cloudSyncContentPrefs: _cloudSyncContentPrefs,
        setCloudSyncContentPrefs
    } = appState;

    const tagManagerSizeRef = useRef<{ width: number; height: number } | null>(null);

    const t = useCallback((key: string) => {
        const k = key as keyof typeof translations["zh"];
        // Defense-in-depth for audit P1-7: never index into an undefined table.
        const table = translations[language] ?? translations["zh"];
        return table[k] || translations["en"][k] || key;
    }, [language]);

    const settings = useSettingsInit({
        setAppSettings,
        setHotkey,
        setTheme,
        setColorMode,
        setCompactMode,
        setLanguage
    });

    useSettingsPostInit({
        settings,
        tagManagerSizeRef,
        setCustomBackground,
        setCustomBackgroundOpacity,
        setSurfaceOpacity,
        setPersistent,
        setPersistentLimitEnabled,
        setPersistentLimit,
        setDeduplicate,
        setCaptureFiles,
        setCaptureRichText,
        setRichTextSnapshotPreview,
        setPrivacyProtection,
        setPrivacyProtectionKinds,
        setPrivacyProtectionCustomRules,
        setSensitiveMaskPrefixVisible,
        setSensitiveMaskSuffixVisible,
        setSensitiveMaskEmailDomain,
        setCleanupRules,
        setAppCleanupPolicies,
        setSilentStart,
        setFollowMouse,
        setShowAppBorder,
        setShowSourceAppIcon,
        setDeleteAfterPaste,
        setMoveToTopAfterPaste,
        setHideTrayIcon,
        setEdgeDocking,
        setCheckUpdateOnStartup,
        setShowSearchBox,
        setScrollTopButtonEnabled,
        setArrowKeySelection,
        setMqttEnabled,
        setMqttServer,
        setRegistryWinVEnabled,
        setMqttPort,
        setMqttUser,
        setMqttPass,
        setMqttTopic,
        setMqttProtocol,
        setMqttWsPath,
        setMqttNotificationEnabled,
        setCloudSyncEnabled,
        setCloudSyncAuto,
        setCloudSyncProvider,
        setCloudSyncServer,
        setCloudSyncApiKey,
        setCloudSyncIntervalSec,
        setCloudSyncSnapshotIntervalMin,
        setCloudSyncWebdavUrl,
        setCloudSyncWebdavUsername,
        setCloudSyncWebdavPassword,
        setCloudSyncWebdavBasePath,
        setCloudSyncE2eEnabled,
        setCloudSyncE2ePassphrase,
        setFileServerAutoClose,
        setFileTransferAutoOpen,
        setFileTransferAutoCopy,
        setFileServerPort,
        setSequentialHotkey,
        setRichPasteHotkey,
        setSearchHotkey,
        setSensitiveHotkey,
        setQuickPasteModifier,
        setQuickPasteInAppEnabled,
        setSequentialModeState,
        setSoundEnabled,
        setSoundVolume,
        setPasteSoundEnabled,
        setPasteMethod,
        setAiEnabled,
        setAiTargetLang,
        setAiThinkingBudget,
        setIsWindowPinned,
        setAiProfiles,
        setAiAssignedProfileTask,
        setAiAssignedProfileMouthpiece,
        setAiAssignedProfileTranslate,
        setSettingsLoaded,
        setClipboardItemFontSize,
        setClipboardTagFontSize,
        setEmojiPanelEnabled,
        setCardDensity,
        setTagManagerEnabled,
        setEmojiPanelTab,
        setEmojiFavorites,
        setHideDockIcon,
        setCloudSyncContentPrefs
    });

    const fetchEffectiveTransferPath = useCallback(() => {
        invoke<string>("get_active_file_transfer_path")
            .then(setFileTransferPath)
            .catch(console.error);
    }, [setFileTransferPath]);

    useAppBootstrap({
        fetchEffectiveTransferPath,
        setDataPath,
        setInstalledApps,
        setAutoStart,
        setWinClipboardDisabled,
        setDefaultApps,
        setFileServerEnabled,
        setActualPort,
        setLocalIp,
        setAvailableIps
    });

    useSettingsApply({
        theme,
        colorMode,
        showAppBorder,
        compactMode,
        cardDensity,
        settingsLoaded,
        clipboardItemFontSize,
        clipboardTagFontSize,
        surfaceOpacity
    });

    useCustomBackground({
        customBackground,
        customBackgroundOpacity,
        theme
    });


    return (
        <div className="advanced-settings-window-shell">
            <AdvancedSettingsGroup
                t={t}
                cleanupRules={cleanupRules}
                setCleanupRules={setCleanupRules}
                appCleanupPolicies={appCleanupPolicies}
                setAppCleanupPolicies={setAppCleanupPolicies}
                installedApps={installedApps}
            />
        </div>
    );
};

export default AdvancedSettingsWindow;
