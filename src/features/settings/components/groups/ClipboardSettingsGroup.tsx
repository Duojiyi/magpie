import { useEffect, useState } from "react";
import type { ComponentType, ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { ChevronDown, ChevronRight, RotateCcw, Clipboard } from "lucide-react";
import { getHotkeyDisplayTokens } from "../../../../shared/lib/hotkeyDisplay";
import { detectPlatform, isMacPlatform, isWindowsPlatform } from "../../../../shared/lib/platform";
import type { QuickPasteModifier } from "../../../app/types";
import type { HotkeyScope } from "../../../../shared/hooks/useHotkeyConfig";
import {
    canShowWinVConflictPrompt,
    dismissWinVConflictForSession,
} from "../../lib/winVConflictSession";

// 可配置作用域的快捷键 id（与后端 `app.hotkey.scope.<id>` 的 id 对齐，需求 19.5）
type ScopedHotkeyId = "main" | "sequential" | "rich" | "search";

interface LabelWithHintProps {
    label: string;
    hint?: string | ReactNode;
    hintKey: string;
}

interface ClipboardSettingsGroupProps {
    t: (key: string) => string;
    collapsed: boolean;
    onToggle: () => void;
    LabelWithHint: ComponentType<LabelWithHintProps>;
    persistent: boolean;
    setPersistent: (val: boolean) => void;
    persistentLimitEnabled: boolean;
    setPersistentLimitEnabled: (val: boolean) => void;
    persistentLimit: number;
    setPersistentLimit: (val: number) => void;
    saveAppSetting: (key: string, val: string) => void;
    deduplicate: boolean;
    setDeduplicate: (val: boolean) => void;
    captureFiles: boolean;
    setCaptureFiles: (val: boolean) => void;
    captureRichText: boolean;
    setCaptureRichText: (val: boolean) => void;
    richTextSnapshotPreview: boolean;
    setRichTextSnapshotPreview: (val: boolean) => void;
    richPasteHotkey: string;
    isRecordingRich: boolean;
    setIsRecordingRich: (val: boolean) => void;
    updateRichPasteHotkey: (key: string) => void;
    searchHotkey: string;
    isRecordingSearch: boolean;
    setIsRecordingSearch: (val: boolean) => void;
    updateSearchHotkey: (key: string) => void;
    sensitiveHotkey: string;
    isRecordingSensitive: boolean;
    setIsRecordingSensitive: (val: boolean) => void;
    updateSensitiveHotkey: (key: string) => void;
    updateHotkeyScope: (id: ScopedHotkeyId, scope: HotkeyScope) => void;
    resetHotkeyScopes: () => void;
    quickPasteModifier: QuickPasteModifier;
    setQuickPasteModifier: (val: QuickPasteModifier) => void;
    quickPasteInAppEnabled: boolean;
    setQuickPasteInAppEnabled: (val: boolean) => void;
    deleteAfterPaste: boolean;
    setDeleteAfterPaste: (val: boolean) => void;
    moveToTopAfterPaste: boolean;
    setMoveToTopAfterPaste: (val: boolean) => void;
    sequentialMode: boolean;
    setSequentialModeState: (val: boolean) => void;
    sequentialHotkey: string;
    isRecordingSequential: boolean;
    setIsRecordingSequential: (val: boolean) => void;
    updateSequentialHotkey: (key: string) => void;
    checkHotkeyConflict: (newHotkey: string, mode: 'main' | 'sequential' | 'rich' | 'search') => boolean;
    privacyProtection: boolean;
    setPrivacyProtection: (val: boolean) => void;
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
    privacyKindsOpen: boolean;
    setPrivacyKindsOpen: (val: boolean) => void;
    privacyRulesOpen: boolean;
    setPrivacyRulesOpen: (val: boolean) => void;
    isRecording: boolean;
    setIsRecording: (val: boolean) => void;
    hotkeyParts: string[];
    updateHotkey: (key: string) => void;
    hotkey: string;
    appSettings: Record<string, string>;
    theme: string;
    colorMode: string;
}

const ClipboardSettingsGroup = (props: ClipboardSettingsGroupProps) => {
    const quickPasteOptions: Array<{ value: QuickPasteModifier; label: string }> = isMacPlatform()
        ? [
            { value: "disabled", label: props.t("quick_paste_modifier_disabled") },
            { value: "ctrl", label: "Control (⌃)" },
            { value: "alt", label: "Option (⌥)" },
            { value: "shift", label: "Shift (⇧)" },
            { value: "win", label: "Command (⌘)" }
        ]
        : [
            { value: "disabled", label: props.t("quick_paste_modifier_disabled") },
            { value: "ctrl", label: props.t("quick_paste_modifier_ctrl") },
            { value: "alt", label: props.t("quick_paste_modifier_alt") },
            { value: "shift", label: props.t("quick_paste_modifier_shift") },
            { value: "win", label: props.t("quick_paste_modifier_win") }
        ];
    const [persistentLimitDraft, setPersistentLimitDraft] = useState(
        props.persistentLimit.toString()
    );
    const [maskSettingsOpen, setMaskSettingsOpen] = useState(false);

    // Win+V 接管（仅 Windows，需求 24）
    // 必须用「是不是 Windows」判定，而不是「不是 macOS」：后者会把 Linux 也算成 Windows，
    // 于是 Linux 上会显示 Win+V 接管开关，而它调用的注册表命令在那边是空实现。
    // 平台以后端 get_platform_info 为准（UA 无法区分 Linux 与 Windows），首帧先用同步猜测。
    const [isWindows, setIsWindows] = useState(isWindowsPlatform());
    useEffect(() => {
        detectPlatform()
            .then((platform) => setIsWindows(platform === "windows"))
            .catch(() => undefined);
    }, []);
    const [winVTakeover, setWinVTakeover] = useState(false);
    // 冲突确认提示的可见性（组件内会话标志由 winVConflictDismissedThisSession 控制是否允许弹出）
    const [winVConflictPrompt, setWinVConflictPrompt] = useState<string | null>(null);

    // 打开面板时读取注册表反推开关状态（需求 24.5/24.6）
    useEffect(() => {
        if (!isWindows) return;
        invoke<boolean>("is_registry_win_v_optimized")
            .then((enabled) => setWinVTakeover(enabled))
            .catch(console.error);
    }, [isWindows]);

    // 探测占用来源应用名（需求 24.8），命令未注册时静默降级
    const detectWinVOccupier = async (): Promise<string | null> => {
        try {
            return await invoke<string | null>("detect_win_v_occupier");
        } catch {
            return null;
        }
    };

    // 写入接管设置：成功后中文提示需手动重启资源管理器（需求 24.3）；
    // 失败且系统占用时弹中文确认提示（需求 24.7），并指明占用来源（需求 24.8）。
    const applyWinVTakeover = async (enable: boolean) => {
        try {
            await invoke<boolean>("trigger_registry_win_v_optimization", { enable });
            setWinVTakeover(enable);
            // 持久化用户选择，使重启后启动逻辑按此决定是否接管 Win+V（默认开启）。
            props.saveAppSetting('use_win_v_shortcut', String(enable));
            if (enable) {
                emit("toast", { msg: props.t("win_v_takeover_success"), variant: "success" }).catch(console.error);
            }
        } catch (err) {
            console.error(err);
            // 注册失败：先探测占用来源
            const occupier = await detectWinVOccupier();
            if (occupier) {
                emit(
                    "toast",
                    { msg: props.t("win_v_occupier_detected").replace("{app}", occupier), variant: "error" }
                ).catch(console.error);
            } else if (canShowWinVConflictPrompt()) {
                // 系统占用且本会话未关闭过提示 → 弹出确认
                setWinVConflictPrompt(props.t("win_v_conflict_prompt"));
            }
        }
    };

    const handleWinVToggle = (next: boolean) => {
        void applyWinVTakeover(next);
    };

    const confirmWinVConflict = () => {
        setWinVConflictPrompt(null);
        void applyWinVTakeover(true);
    };

    const dismissWinVConflict = () => {
        // 同会话内关闭后不再重复弹出（需求 24.7）
        dismissWinVConflictForSession();
        setWinVConflictPrompt(null);
    };

    useEffect(() => {
        setPersistentLimitDraft(props.persistentLimit.toString());
    }, [props.persistentLimit, props.persistentLimitEnabled]);

    const commitPersistentLimit = (rawValue?: string) => {
        const source = rawValue ?? persistentLimitDraft;
        const parsed = parseInt(source, 10);
        if (!Number.isFinite(parsed)) {
            setPersistentLimitDraft(props.persistentLimit.toString());
            return;
        }
        const clamped = Math.max(50, Math.min(99999, parsed));
        props.setPersistentLimit(clamped);
        props.saveAppSetting('persistent_limit', clamped.toString());
        if (clamped.toString() !== source) {
            setPersistentLimitDraft(clamped.toString());
        }
    };

    const renderHotkeyCaps = (hotkey: string) => {
        const tokens = getHotkeyDisplayTokens(hotkey, { preferMacSymbols: true });
        if (tokens.length === 0) {
            return <div className="key-cap" style={{ width: '8em', opacity: 0.5 }}>{props.t('not_set')}</div>;
        }
        const compactLabel = tokens.map((token) => token.label).join("");
        return <div className="key-cap key-cap-chord">{compactLabel}</div>;
    };

    // 单个快捷键的作用域选择控件（需求 19.5）：从 appSettings 读取当前值，
    // 缺省按 Global 兜底（与后端 parse_scope 一致，需求 19.4）；
    // 改动后调用 updateHotkeyScope 持久化并触发即时重新分流（需求 19.9）。
    const renderScopeSelect = (id: ScopedHotkeyId) => {
        const current = (props.appSettings[`app.hotkey.scope.${id}`] as HotkeyScope) || "Global";
        return (
            <select
                value={current}
                onChange={(e) => props.updateHotkeyScope(id, e.target.value as HotkeyScope)}
                style={{
                    padding: '4px 8px',
                    borderRadius: '4px',
                    border: '1px solid var(--border-color)',
                    background: 'var(--input-bg)',
                    color: 'var(--text-color)',
                    fontSize: '13px',
                    minWidth: '120px'
                }}
            >
                <option value="Global">{props.t('hotkey_scope_global')}</option>
                <option value="InAppOnly">{props.t('hotkey_scope_in_app_only')}</option>
                <option value="BackgroundOnly">{props.t('hotkey_scope_background_only')}</option>
            </select>
        );
    };

    // 快捷键作用域选择项（标签 + 下拉），紧跟在对应快捷键录制控件之后（需求 19.5）。
    const renderScopeRow = (id: ScopedHotkeyId) => (
        <div className="setting-item">
            <props.LabelWithHint
                label={props.t('hotkey_scope_label')}
                hint={props.t('hotkey_scope_hint')}
                hintKey={`hotkey_scope_${id}`}
            />
            {renderScopeSelect(id)}
        </div>
    );

    return (
        <div className={`settings-group ${props.collapsed ? 'collapsed' : ''}`}>
            <div className="group-header" onClick={props.onToggle}>
                {/* 标题区统一使用 lucide 图标（需求 30.1/30.2） */}
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <Clipboard size={16} />
                    <h3 style={{ margin: 0 }}>{props.t('clipboard_settings')}</h3>
                </div>
                {props.collapsed ? <ChevronRight size={16} /> : <ChevronDown size={16} />}
            </div>
            {!props.collapsed && (
                <div className="group-content">
                    <div className="setting-item">
                        <props.LabelWithHint
                            label={props.t('persistent_storage')}
                            hint={props.t('persistent_hint')}
                            hintKey="persistent_storage"
                        />
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={props.persistent}
                                onChange={(e) => props.setPersistent(e.target.checked)}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>
                    {props.persistent && (
                        <>
                            <div className="setting-item">
                                <props.LabelWithHint
                                    label={props.t('persistent_limit_enabled')}
                                    hint={props.t('persistent_limit_enabled_hint')}
                                    hintKey="persistent_limit_enabled"
                                />
                                <label className="switch">
                                    <input
                                        className="cb"
                                        type="checkbox"
                                        checked={props.persistentLimitEnabled}
                                        onChange={(e) => {
                                            props.setPersistentLimitEnabled(e.target.checked);
                                            props.saveAppSetting('persistent_limit_enabled', e.target.checked.toString());
                                        }}
                                    />
                                    <div className="toggle"><div className="left" /><div className="right" /></div>
                                </label>
                            </div>
                            {props.persistentLimitEnabled && (
                                <div className="setting-item">
                                    <props.LabelWithHint
                                        label={props.t('persistent_limit')}
                                        hint={props.t('persistent_limit_hint')}
                                        hintKey="persistent_limit"
                                    />
                                    <input
                                        type="number"
                                        value={persistentLimitDraft}
                                        onFocus={(e) => {
                                            e.target.select();
                                            invoke("focus_clipboard_window").catch(console.error);
                                        }}
                                        onChange={(e) => {
                                            const next = e.target.value;
                                            if (next === "") {
                                                setPersistentLimitDraft("");
                                                return;
                                            }
                                            if (!/^\d+$/.test(next)) return;
                                            setPersistentLimitDraft(next);
                                        }}
                                        onBlur={() => {
                                            commitPersistentLimit();
                                        }}
                                        onKeyDown={(e) => {
                                            if (e.key === 'Enter') {
                                                commitPersistentLimit(e.currentTarget.value);
                                                e.currentTarget.blur();
                                            }
                                        }}
                                        style={{
                                            width: '90px',
                                            padding: '4px 8px',
                                            borderRadius: '4px',
                                            border: '1px solid var(--border-color)',
                                            background: 'var(--input-bg)',
                                            color: 'var(--text-color)',
                                            fontSize: '14px'
                                        }}
                                    />
                                </div>
                            )}
                        </>
                    )}
                    <div className="setting-item">
                        <props.LabelWithHint
                            label={props.t('merge_duplicates')}
                            hint={props.t('merge_duplicates_hint') || "Time limit to prevent accidental multiple copies"}
                            hintKey="merge_duplicates"
                        />
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={props.deduplicate}
                                onChange={(e) => props.setDeduplicate(e.target.checked)}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>
                    <div className="setting-item">
                        <div className="item-label-group">
                            <span className="item-label">{props.t('capture_files')}</span>
                        </div>
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={props.captureFiles}
                                onChange={(e) => props.setCaptureFiles(e.target.checked)}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>
                    <div className="setting-item">
                        <props.LabelWithHint
                            label={props.t('capture_rich_text') || '捕获富文本'}
                            hint={props.t('capture_rich_text_hint') || '开启后可记录富文本并支持双击带格式粘贴'}
                            hintKey="capture_rich_text"
                        />
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={props.captureRichText}
                                onChange={(e) => {
                                    const val = e.target.checked;
                                    props.setCaptureRichText(val);
                                }}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>
                    <div className="setting-item">
                        <props.LabelWithHint
                            label={props.t('rich_text_snapshot_preview') || '富文本快照预览'}
                            hint={props.t('rich_text_snapshot_preview_hint') || '开启后将富文本转换为内存快照图用于条目与悬浮预览'}
                            hintKey="rich_text_snapshot_preview"
                        />
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={props.richTextSnapshotPreview}
                                onChange={(e) => {
                                    const val = e.target.checked;
                                    props.setRichTextSnapshotPreview(val);
                                    props.saveAppSetting('rich_text_snapshot_preview', String(val));
                                }}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>


                    <div className="setting-item">
                        <div className="item-label-group">
                            <span className="item-label">{props.t('rich_paste_hotkey_label')}</span>
                            <span className="hint">{props.isRecordingRich ? props.t('hotkey_recording_esc') : props.t('hotkey_click_hint')}</span>
                        </div>
                        <div
                            className={`key-group ${props.isRecordingRich ? 'recording' : ''}`}
                            onClick={(e) => { props.setIsRecordingRich(true); e.currentTarget.focus(); }}
                            tabIndex={0}
                            onKeyDown={(e) => {
                                if (!props.isRecordingRich) return;
                                e.preventDefault();
                                e.stopPropagation();

                                if (e.key === 'Escape') {
                                    props.setIsRecordingRich(false);
                                    return;
                                }

                                if (e.key === 'Backspace' || e.key === 'Delete') {
                                    props.updateRichPasteHotkey('');
                                    props.setIsRecordingRich(false);
                                    return;
                                }

                                const modifiers = [];
                                if (e.ctrlKey) modifiers.push('Ctrl');
                                if (e.shiftKey) modifiers.push('Shift');
                                if (e.altKey) modifiers.push('Alt');
                                if (e.metaKey) modifiers.push('Win');

                                const key = e.key.toUpperCase();
                                if (['CONTROL', 'SHIFT', 'ALT', 'META'].includes(key)) return;

                                const newHotkey = [...modifiers, key].join('+');
                                props.updateRichPasteHotkey(newHotkey);
                            }}
                        >
                            {props.isRecordingRich ? (
                                <div className="key-cap" style={{ width: '8em' }}>{props.t('waiting_for_input')}</div>
                            ) : (
                                renderHotkeyCaps(props.richPasteHotkey)
                            )}
                        </div>
                    </div>
                    {renderScopeRow('rich')}
                    <div className="setting-item">
                        <div className="item-label-group">
                            <span className="item-label">{props.t('search_hotkey_label')}</span>
                            <span className="hint">{props.isRecordingSearch ? props.t('hotkey_recording_esc') : props.t('hotkey_click_hint')}</span>
                        </div>
                        <div
                            className={`key-group ${props.isRecordingSearch ? 'recording' : ''}`}
                            onClick={(e) => { props.setIsRecordingSearch(true); e.currentTarget.focus(); }}
                            tabIndex={0}
                            onKeyDown={(e) => {
                                if (!props.isRecordingSearch) return;
                                e.preventDefault();
                                e.stopPropagation();

                                if (e.key === 'Escape') {
                                    props.setIsRecordingSearch(false);
                                    return;
                                }

                                if (e.key === 'Backspace' || e.key === 'Delete') {
                                    props.updateSearchHotkey('');
                                    props.setIsRecordingSearch(false);
                                    return;
                                }

                                const modifiers = [];
                                if (e.ctrlKey) modifiers.push('Ctrl');
                                if (e.shiftKey) modifiers.push('Shift');
                                if (e.altKey) modifiers.push('Alt');
                                if (e.metaKey) modifiers.push('Win');

                                const key = e.key.toUpperCase();
                                if (['CONTROL', 'SHIFT', 'ALT', 'META'].includes(key)) return;

                                const newHotkey = [...modifiers, key].join('+');
                                props.updateSearchHotkey(newHotkey);
                            }}
                        >
                            {props.isRecordingSearch ? (
                                <div className="key-cap" style={{ width: '8em' }}>{props.t('waiting_for_input')}</div>
                            ) : (
                                renderHotkeyCaps(props.searchHotkey)
                            )}
                        </div>
                    </div>
                    {renderScopeRow('search')}
                    {/* 敏感标记快捷键（需求 17.3）：默认 S，可自定义覆盖。
                        Scope 固定 InAppOnly（仅主面板可见时由 useKeyboardNavigation 的 webview keydown 响应），
                        因此不提供 Scope 选择行。 */}
                    <div className="setting-item">
                        <div className="item-label-group">
                            <span className="item-label">{props.t('sensitive_hotkey_label')}</span>
                            <span className="hint">{props.isRecordingSensitive ? props.t('hotkey_recording_esc') : props.t('hotkey_click_hint')}</span>
                        </div>
                        <div
                            className={`key-group ${props.isRecordingSensitive ? 'recording' : ''}`}
                            onClick={(e) => { props.setIsRecordingSensitive(true); e.currentTarget.focus(); }}
                            tabIndex={0}
                            onKeyDown={(e) => {
                                if (!props.isRecordingSensitive) return;
                                e.preventDefault();
                                e.stopPropagation();

                                if (e.key === 'Escape') {
                                    props.setIsRecordingSensitive(false);
                                    return;
                                }

                                if (e.key === 'Backspace' || e.key === 'Delete') {
                                    props.updateSensitiveHotkey('');
                                    props.setIsRecordingSensitive(false);
                                    return;
                                }

                                const modifiers = [];
                                if (e.ctrlKey) modifiers.push('Ctrl');
                                if (e.shiftKey) modifiers.push('Shift');
                                if (e.altKey) modifiers.push('Alt');
                                if (e.metaKey) modifiers.push('Win');

                                const key = e.key.toUpperCase();
                                if (['CONTROL', 'SHIFT', 'ALT', 'META'].includes(key)) return;

                                const newHotkey = [...modifiers, key].join('+');
                                props.updateSensitiveHotkey(newHotkey);
                            }}
                        >
                            {props.isRecordingSensitive ? (
                                <div className="key-cap" style={{ width: '8em' }}>{props.t('waiting_for_input')}</div>
                            ) : (
                                renderHotkeyCaps(props.sensitiveHotkey)
                            )}
                        </div>
                    </div>
                    <div className="setting-item">
                        <props.LabelWithHint
                            label={props.t('quick_paste_modifier')}
                            hint={props.t('quick_paste_modifier_hint')}
                            hintKey="quick_paste_modifier"
                        />
                        <select
                            value={props.quickPasteModifier}
                            onChange={(e) => {
                                const value = e.target.value as QuickPasteModifier;
                                props.setQuickPasteModifier(value);
                                invoke("set_quick_paste_modifier", { modifier: value }).catch(console.error);
                            }}
                            style={{
                                padding: '4px 8px',
                                borderRadius: '4px',
                                border: '1px solid var(--border-color)',
                                background: 'var(--input-bg)',
                                color: 'var(--text-color)',
                                fontSize: '14px',
                                minWidth: '140px'
                            }}
                        >
                            {quickPasteOptions.map((option) => (
                                <option key={option.value} value={option.value}>
                                    {option.label}
                                </option>
                            ))}
                        </select>
                    </div>
                    {/* 数字快捷粘贴（Ctrl+1~9，InAppOnly）开关（需求 16.1）。
                        启用后由 useKeyboardNavigation 在主面板可见时响应 Ctrl+1~9，按过滤后可见列表第 N 个粘贴；
                        粘贴成功后由后端 copy_to_clipboard 隐藏主面板（需求 16.4）。
                        与旧的 quick_paste_modifier（基于置顶项）并存兼容。 */}
                    <div className="setting-item">
                        <props.LabelWithHint
                            label={props.t('quick_paste_in_app_enabled')}
                            hint={props.t('quick_paste_in_app_enabled_hint')}
                            hintKey="quick_paste_in_app_enabled"
                        />
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={props.quickPasteInAppEnabled}
                                onChange={(e) => {
                                    const val = e.target.checked;
                                    props.setQuickPasteInAppEnabled(val);
                                    props.saveAppSetting('quick_paste_in_app_enabled', String(val));
                                }}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>
                    <div className="setting-item">
                        <props.LabelWithHint
                            label={props.t('delete_after_paste')}
                            hint={props.t('delete_after_paste_hint')}
                            hintKey="delete_after_paste"
                        />
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={props.deleteAfterPaste}
                                onChange={(e) => {
                                    const val = e.target.checked;
                                    props.setDeleteAfterPaste(val);
                                    props.saveAppSetting('delete_after_paste', String(val));
                                }}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>
                    <div className="setting-item">
                        <props.LabelWithHint
                            label={props.t('move_to_top_after_paste')}
                            hint={props.t('move_to_top_after_paste_hint')}
                            hintKey="move_to_top_after_paste"
                        />
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={props.moveToTopAfterPaste}
                                onChange={(e) => {
                                    const val = e.target.checked;
                                    props.setMoveToTopAfterPaste(val);
                                    props.saveAppSetting('move_to_top_after_paste', String(val));
                                }}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>
                    {/* macOS cleanup: Removed Paste Method selection */}
                    <div className="setting-item">
                        <props.LabelWithHint
                            label={props.t('sequential_paste_mode')}
                            hint={props.t('sequential_paste_hint')}
                            hintKey="sequential_paste_mode"
                        />
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={props.sequentialMode}
                                onChange={(e) => {
                                    const val = e.target.checked;
                                    props.setSequentialModeState(val);
                                    invoke('set_sequential_mode', { enabled: val }).catch(console.error);
                                    if (val) {
                                        if (props.checkHotkeyConflict(props.sequentialHotkey, 'sequential')) {
                                            props.updateSequentialHotkey("");
                                        }
                                    }
                                }}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>

                    {props.sequentialMode && (
                        <div className="setting-item">
                            <div className="item-label-group">
                                <span className="item-label">{props.t('sequential_paste_hotkey_label')}</span>
                                <span className="hint">{props.isRecordingSequential ? props.t('hotkey_recording_esc') : props.t('hotkey_click_hint')}</span>
                            </div>
                            <div
                                className={`key-group ${props.isRecordingSequential ? 'recording' : ''}`}
                                onClick={(e) => { props.setIsRecordingSequential(true); e.currentTarget.focus(); }}
                                tabIndex={0}
                                onKeyDown={(e) => {
                                    if (!props.isRecordingSequential) return;
                                    e.preventDefault();
                                    e.stopPropagation();

                                    if (e.key === 'Escape') {
                                        props.setIsRecordingSequential(false);
                                        return;
                                    }

                                    if (e.key === 'Backspace' || e.key === 'Delete') {
                                        props.updateSequentialHotkey('');
                                        props.setIsRecordingSequential(false);
                                        return;
                                    }

                                    const modifiers = [];
                                    if (e.ctrlKey) modifiers.push('Ctrl');
                                    if (e.shiftKey) modifiers.push('Shift');
                                    if (e.altKey) modifiers.push('Alt');
                                    if (e.metaKey) modifiers.push('Win');

                                    const key = e.key.toUpperCase();
                                    if (['CONTROL', 'SHIFT', 'ALT', 'META'].includes(key)) return;

                                    const newHotkey = [...modifiers, key].join('+');
                                    props.updateSequentialHotkey(newHotkey);
                                }}
                            >
                                {props.isRecordingSequential ? (
                                    <div className="key-cap" style={{ width: '8em' }}>{props.t('waiting_for_input')}</div>
                                ) : (
                                    renderHotkeyCaps(props.sequentialHotkey)
                                )}
                            </div>
                        </div>
                    )}

                    {props.sequentialMode && renderScopeRow('sequential')}

                    <div className="setting-item">
                        <props.LabelWithHint
                            label={props.t('privacy_protection')}
                            hint={props.t('privacy_protection_hint')}
                            hintKey="privacy_protection"
                        />
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={props.privacyProtection}
                                onChange={(e) => {
                                    const val = e.target.checked;
                                    props.setPrivacyProtection(val);
                                    invoke('set_privacy_protection', { enabled: val }).catch(console.error);
                                }}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>

                    <div className="setting-item" style={{ flexDirection: 'column', alignItems: 'flex-start', gap: '6px' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                            <button
                                type="button"
                                className="btn-icon"
                                onClick={() => props.setPrivacyKindsOpen(!props.privacyKindsOpen)}
                                style={{ width: '24px', height: '24px' }}
                            >
                                {props.privacyKindsOpen ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                            </button>
                            <props.LabelWithHint
                                label={props.t('privacy_protection_kinds')}
                                hint={props.t('privacy_protection_kinds_hint')}
                                hintKey="privacy_protection_kinds"
                            />
                        </div>
                        {props.privacyKindsOpen && (
                            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', marginLeft: '30px' }}>
                                {[
                                    { id: 'url', label: props.t('privacy_kind_url') || '链接 / URL' },
                                    { id: 'phone', label: props.t('privacy_kind_phone') },
                                    { id: 'idcard', label: props.t('privacy_kind_idcard') },
                                    { id: 'email', label: props.t('privacy_kind_email') },
                                    { id: 'secret', label: props.t('privacy_kind_secret') },
                                    { id: 'password', label: props.t('privacy_kind_password') || "Strong Password" },
                                ].map(opt => {
                                    const checked = props.privacyProtectionKinds.includes(opt.id);
                                    return (
                                        <label key={opt.id} style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                                            <input
                                                className="cb"
                                                type="checkbox"
                                                checked={checked}
                                                onChange={(e) => {
                                                    const next = e.target.checked
                                                        ? [...props.privacyProtectionKinds, opt.id]
                                                        : props.privacyProtectionKinds.filter(t => t !== opt.id);
                                                    props.setPrivacyProtectionKinds(next);
                                                    invoke('set_privacy_protection_kinds', { kinds: next }).catch(console.error);
                                                }}
                                            />
                                            <span style={{ fontSize: '12px', color: 'var(--text-primary)' }}>{opt.label}</span>
                                        </label>
                                    );
                                })}
                            </div>
                        )}
                    </div>

                    <div className="setting-item" style={{ flexDirection: 'column', alignItems: 'flex-start', gap: '6px' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                            <button
                                type="button"
                                className="btn-icon"
                                onClick={() => props.setPrivacyRulesOpen(!props.privacyRulesOpen)}
                                style={{ width: '24px', height: '24px' }}
                            >
                                {props.privacyRulesOpen ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                            </button>
                            <props.LabelWithHint
                                label={props.t('privacy_protection_custom_rules')}
                                hint={props.t('privacy_protection_custom_rules_hint')}
                                hintKey="privacy_protection_custom_rules"
                            />
                        </div>
                        {props.privacyRulesOpen && (
                            <textarea
                                className="search-input"
                                style={{ width: 'calc(100% - 30px)', maxWidth: '100%', minHeight: '80px', padding: '8px', borderRadius: '0', marginLeft: '30px', boxSizing: 'border-box' }}
                                placeholder={props.t('privacy_protection_custom_rules_placeholder')}
                                value={props.privacyProtectionCustomRules}
                                onFocus={() => invoke("focus_clipboard_window").catch(console.error)}
                                onChange={(e) => {
                                    const val = e.target.value;
                                    props.setPrivacyProtectionCustomRules(val);
                                    invoke('set_privacy_protection_custom_rules', { rules: val }).catch(console.error);
                                }}
                            />
                        )}
                    </div>

                    <div className="setting-item" style={{ flexDirection: 'column', alignItems: 'flex-start', gap: '6px' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                            <button
                                type="button"
                                className="btn-icon"
                                onClick={() => setMaskSettingsOpen(!maskSettingsOpen)}
                                style={{ width: '24px', height: '24px' }}
                            >
                                {maskSettingsOpen ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                            </button>
                            <span className="item-label">{props.t('sensitive_mask_settings')}</span>
                        </div>
                        {maskSettingsOpen && (
                            <div style={{ width: 'calc(100% - 30px)', marginLeft: '30px', display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                <div className="setting-item" style={{ padding: 0, borderBottom: 'none' }}>
                                    <span className="item-label">{props.t('sensitive_mask_prefix_visible')}</span>
                                    <input
                                        type="number"
                                        className="search-input"
                                        style={{ width: '60px', padding: '4px 8px', textAlign: 'center' }}
                                        min={0}
                                        max={20}
                                        value={props.sensitiveMaskPrefixVisible}
                                        onChange={(e) => {
                                            const val = Math.min(20, Math.max(0, parseInt(e.target.value) || 0));
                                            props.setSensitiveMaskPrefixVisible(val);
                                            invoke('save_setting', { key: 'app.sensitive_mask_prefix_visible', value: val.toString() }).catch(console.error);
                                        }}
                                    />
                                </div>
                                <div className="setting-item" style={{ padding: 0, borderBottom: 'none' }}>
                                    <span className="item-label">{props.t('sensitive_mask_suffix_visible')}</span>
                                    <input
                                        type="number"
                                        className="search-input"
                                        style={{ width: '60px', padding: '4px 8px', textAlign: 'center' }}
                                        min={0}
                                        max={20}
                                        value={props.sensitiveMaskSuffixVisible}
                                        onChange={(e) => {
                                            const val = Math.min(20, Math.max(0, parseInt(e.target.value) || 0));
                                            props.setSensitiveMaskSuffixVisible(val);
                                            invoke('save_setting', { key: 'app.sensitive_mask_suffix_visible', value: val.toString() }).catch(console.error);
                                        }}
                                    />
                                </div>
                                <div className="setting-item" style={{ padding: 0, borderBottom: 'none' }}>
                                    <props.LabelWithHint
                                        label={props.t('sensitive_mask_email_domain')}
                                        hint={props.t('sensitive_mask_email_domain_hint')}
                                        hintKey="sensitive_mask_email_domain"
                                    />
                                    <label className="switch">
                                        <input
                                            type="checkbox"
                                            checked={props.sensitiveMaskEmailDomain}
                                            onChange={(e) => {
                                                props.setSensitiveMaskEmailDomain(e.target.checked);
                                                invoke('save_setting', { key: 'app.sensitive_mask_email_domain', value: e.target.checked.toString() }).catch(console.error);
                                            }}
                                        />
                                        <span className="slider" />
                                    </label>
                                </div>
                            </div>
                        )}
                    </div>

                    <div className="setting-item no-border">
                        <div className="item-label-group">
                            <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                                <span className="item-label">{props.t('global_hotkey')}</span>
                            </div>
                            <span className="hint">{props.isRecording ? props.t('hotkey_recording_esc') : props.t('hotkey_click_hint')}</span>
                        </div>

                        <div
                            className={`key-group ${props.isRecording ? 'recording' : ''}`}
                            onClick={(e) => { props.setIsRecording(true); e.currentTarget.focus(); }}
                            tabIndex={0}
                            onKeyDown={(e) => {
                                if (!props.isRecording) return;
                                e.preventDefault();
                                e.stopPropagation();

                                if (e.key === 'Escape') {
                                    props.setIsRecording(false);
                                    return;
                                }

                                if (e.key === 'Backspace' || e.key === 'Delete') {
                                    props.updateHotkey('');
                                    props.setIsRecording(false);
                                    return;
                                }

                                const modifiers = [];
                                if (e.ctrlKey) modifiers.push('Ctrl');
                                if (e.shiftKey) modifiers.push('Shift');
                                if (e.altKey) modifiers.push('Alt');
                                if (e.metaKey) modifiers.push('Win');

                                const key = e.key.toUpperCase();
                                if (['CONTROL', 'SHIFT', 'ALT', 'META'].includes(key)) return;

                                const newHotkey = [...modifiers, key].join('+');
                                props.updateHotkey(newHotkey);
                            }}
                        >
                            {props.isRecording ? (
                                <div className="key-cap" style={{ width: '8em' }}>{props.t('waiting_for_input')}</div>
                            ) : (
                                renderHotkeyCaps(props.hotkey)
                            )}
                        </div>
                    </div>
                    {renderScopeRow('main')}

                    {/* 恢复默认：将所有快捷键作用域还原为默认值（既有快捷键默认 Global，需求 19.6） */}
                    <div className="setting-item">
                        <props.LabelWithHint
                            label={props.t('hotkey_scope_reset_label')}
                            hint={props.t('hotkey_scope_reset_hint')}
                            hintKey="hotkey_scope_reset"
                        />
                        <button
                            type="button"
                            className="btn-icon"
                            style={{ width: 'auto', fontSize: '12px', height: '28px', padding: '0 12px', display: 'inline-flex', alignItems: 'center', gap: '6px' }}
                            onClick={() => props.resetHotkeyScopes()}
                        >
                            <RotateCcw size={14} />
                            {props.t('hotkey_scope_reset_button')}
                        </button>
                    </div>

                    {/* Win+V 接管开关（仅 Windows，需求 24） */}
                    {isWindows && (
                        <div className="setting-item" style={{ flexDirection: 'column', alignItems: 'stretch', gap: '8px' }}>
                            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
                                <props.LabelWithHint
                                    label={props.t('win_v_takeover')}
                                    hint={props.t('win_v_takeover_hint')}
                                    hintKey="win_v_takeover"
                                />
                                <label className="switch">
                                    <input
                                        className="cb"
                                        type="checkbox"
                                        checked={winVTakeover}
                                        onChange={(e) => handleWinVToggle(e.target.checked)}
                                    />
                                    <div className="toggle"><div className="left" /><div className="right" /></div>
                                </label>
                            </div>
                            {winVConflictPrompt && (
                                <div
                                    style={{
                                        display: 'flex',
                                        alignItems: 'center',
                                        justifyContent: 'space-between',
                                        gap: '8px',
                                        padding: '8px 10px',
                                        borderRadius: '6px',
                                        background: 'var(--input-bg)',
                                        border: '1px solid var(--border-color)'
                                    }}
                                >
                                    <span style={{ fontSize: '12px', color: 'var(--text-primary)', flex: 1 }}>
                                        {winVConflictPrompt}
                                    </span>
                                    <div style={{ display: 'flex', gap: '6px', flexShrink: 0 }}>
                                        <button
                                            type="button"
                                            className="btn-icon"
                                            style={{ width: 'auto', fontSize: '11px', height: '26px', padding: '0 12px' }}
                                            onClick={confirmWinVConflict}
                                        >
                                            {props.t('confirm')}
                                        </button>
                                        <button
                                            type="button"
                                            className="btn-icon"
                                            style={{ width: 'auto', fontSize: '11px', height: '26px', padding: '0 12px' }}
                                            onClick={dismissWinVConflict}
                                        >
                                            {props.t('cancel')}
                                        </button>
                                    </div>
                                </div>
                            )}
                        </div>
                    )}
                </div>
            )}
        </div>
    );
};

export default ClipboardSettingsGroup;
