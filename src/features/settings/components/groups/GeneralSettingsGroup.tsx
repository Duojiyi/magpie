import type { ComponentType, ReactNode } from "react";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { ChevronDown, ChevronRight, ClipboardCopy, Settings } from "lucide-react";
import type { CardDensity } from "../../../app/types";

const isMacPlatform =
    /Mac|iPhone|iPad|iPod/i.test(navigator.userAgent) || /Mac/i.test(navigator.platform);

interface LabelWithHintProps {
    label: string;
    hint?: string | ReactNode;
    hintKey: string;
}

interface GeneralSettingsGroupProps {
    t: (key: string) => string;
    collapsed: boolean;
    onToggle: () => void;
    LabelWithHint: ComponentType<LabelWithHintProps>;
    autoStart: boolean;
    setAutoStart: (val: boolean) => void;
    silentStart: boolean;
    setSilentStart: (val: boolean) => void;
    hideTrayIcon: boolean;
    setHideTrayIcon: (val: boolean) => void;
    hideDockIcon: boolean;
    setHideDockIcon: (val: boolean) => void;
    edgeDocking: boolean;
    setEdgeDocking: (val: boolean) => void;
    checkUpdateOnStartup: boolean;
    setCheckUpdateOnStartup: (val: boolean) => void;
    soundEnabled: boolean;
    setSoundEnabled: (val: boolean) => void;
    pasteSoundEnabled: boolean;
    setPasteSoundEnabled: (val: boolean) => void;
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
    soundVolume: number;
    setSoundVolume: (val: number) => void;
    saveAppSetting: (key: string, val: string) => void;
}

const GeneralSettingsGroup = ({
    t,
    collapsed,
    onToggle,
    LabelWithHint,
    autoStart,
    setAutoStart,
    silentStart,
    setSilentStart,
    hideTrayIcon,
    setHideTrayIcon,
    hideDockIcon,
    setHideDockIcon,
    edgeDocking,
    setEdgeDocking,
    checkUpdateOnStartup,
    setCheckUpdateOnStartup,
    soundEnabled,
    setSoundEnabled,
    pasteSoundEnabled,
    setPasteSoundEnabled,
    showSearchBox,
    setShowSearchBox,
    scrollTopButtonEnabled,
    setScrollTopButtonEnabled,
    emojiPanelEnabled,
    setEmojiPanelEnabled,
    cardDensity,
    setCardDensity,
    tagManagerEnabled,
    setTagManagerEnabled,
    arrowKeySelection,
    setArrowKeySelection,
    soundVolume,
    setSoundVolume,
    saveAppSetting
}: GeneralSettingsGroupProps) => {
    // 「复制诊断信息」按钮的即时反馈状态：idle 空闲 / copied 成功 / error 失败
    const [diagState, setDiagState] = useState<'idle' | 'copied' | 'error'>('idle');

    // 调用后端 copy_diagnostics 收集并脱敏诊断信息，再写入剪贴板（不做任何网络上传）
    const handleCopyDiagnostics = async () => {
        try {
            const report = await invoke<string>("copy_diagnostics");
            await navigator.clipboard.writeText(report);
            setDiagState('copied');
        } catch (err) {
            console.error("复制诊断信息失败:", err);
            setDiagState('error');
        }
        window.setTimeout(() => setDiagState('idle'), 2000);
    };

    return (
    <div className={`settings-group ${collapsed ? 'collapsed' : ''}`}>
        <div className="group-header" onClick={onToggle}>
            {/* 标题区统一使用 lucide 图标（需求 30.1/30.2） */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Settings size={16} />
                <h3 style={{ margin: 0 }}>{t('general_settings')}</h3>
            </div>
            {collapsed ? <ChevronRight size={16} /> : <ChevronDown size={16} />}
        </div>
        {!collapsed && (
            <div className="group-content">
                <div className="setting-item">
                    <div className="item-label-group">
                        <span className="item-label">{t('autostart')}</span>
                    </div>
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={autoStart}
                            onChange={(e) => {
                                const enabled = e.target.checked;
                                setAutoStart(enabled);
                                invoke("toggle_autostart", { enabled }).catch((err) => {
                                    // Registration failed: put the switch back and say why.
                                    console.error(err);
                                    setAutoStart(!enabled);
                                    emit("toast", { msg: String(err), variant: "error" }).catch(console.error);
                                });
                            }}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>

                <div className="setting-item">
                    <div className="item-label-group">
                        <span className="item-label">{t('hide_tray_icon')}</span>
                    </div>
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={hideTrayIcon}
                            onChange={(e) => {
                                const val = e.target.checked;
                                setHideTrayIcon(val);
                                invoke("set_tray_visible", { visible: !val }).catch((err) => {
                                    console.error(err);
                                    setHideTrayIcon(!val);
                                    emit("toast", { msg: String(err), variant: "error" }).catch(console.error);
                                });
                            }}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>

                {isMacPlatform && (
                    <div className="setting-item">
                        <LabelWithHint
                            label={t('hide_dock_icon')}
                            hint={t('hide_dock_icon_hint')}
                            hintKey="hide_dock_icon"
                        />
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={hideDockIcon}
                                onChange={(e) => {
                                    const val = e.target.checked;
                                    setHideDockIcon(val);
                                    invoke("set_dock_visible", { visible: !val }).catch((err) => {
                                        console.error(err);
                                        setHideDockIcon(!val);
                                        emit("toast", { msg: String(err), variant: "error" }).catch(console.error);
                                    });
                                }}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>
                )}

                <div className="setting-item">
                    <LabelWithHint
                        label={t('edge_docking')}
                        hint={t('edge_docking_hint')}
                        hintKey="edge_docking"
                    />
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={edgeDocking}
                            onChange={(e) => {
                                const val = e.target.checked;
                                setEdgeDocking(val);
                                invoke("set_edge_docking", { enabled: val }).catch(console.error);
                            }}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>


                <div className="setting-item">
                    <div className="item-label-group">
                        <span className="item-label">{t('sound_effects') || "Sound Effects"}</span>
                    </div>
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={soundEnabled}
                            onChange={(e) => {
                                const enabled = e.target.checked;
                                setSoundEnabled(enabled);
                                invoke("set_sound_enabled", { enabled }).catch(console.error);
                            }}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>
                {soundEnabled && (
                    <div className="setting-item" style={{ marginLeft: '18px' }}>
                        <div className="item-label-group">
                            <span className="item-label">{t('paste_sound') || "Paste Sound"}</span>
                        </div>
                        <label className="switch">
                            <input
                                className="cb"
                                type="checkbox"
                                checked={pasteSoundEnabled}
                                onChange={(e) => {
                                    const enabled = e.target.checked;
                                    setPasteSoundEnabled(enabled);
                                    invoke("save_setting", { key: 'app.sound_paste_enabled', value: String(enabled) }).catch(console.error);
                                }}
                            />
                            <div className="toggle"><div className="left" /><div className="right" /></div>
                        </label>
                    </div>
                )}
                {soundEnabled && (
                    <div className="setting-item column" style={{ marginLeft: '18px', borderBottom: 'none' }}>
                        <div className="item-label-group">
                            <span className="item-label">{t('sound_volume') || "Sound Volume"} ({Math.round(soundVolume * 100)}%)</span>
                        </div>
                        <div style={{ padding: '0 4px', width: '100%' }}>
                            <input
                                type="range"
                                min="0"
                                max="1"
                                step="0.01"
                                value={soundVolume}
                                onChange={(e) => {
                                    const val = parseFloat(e.target.value);
                                    setSoundVolume(val);
                                }}
                                style={{
                                    ['--range-progress' as any]: `${soundVolume * 100}%`
                                }}
                            />
                        </div>
                    </div>
                )}


                <div className="setting-item">
                    <LabelWithHint
                        label={t('silent_start')}
                        hint={t('silent_start_hint')}
                        hintKey="silent_start"
                    />
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={silentStart}
                            onChange={(e) => {
                                const enabled = e.target.checked;
                                setSilentStart(enabled);
                                invoke("set_silent_start", { enabled }).catch(console.error);
                            }}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('check_update_on_startup') || '启动时检查更新'}
                        hint={t('check_update_on_startup_hint') || '关闭后启动时不再向 GitHub 发起更新检查请求，可在此面板手动检查'}
                        hintKey="check_update_on_startup"
                    />
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={checkUpdateOnStartup}
                            onChange={(e) => {
                                const enabled = e.target.checked;
                                setCheckUpdateOnStartup(enabled);
                                saveAppSetting('check_update_on_startup', String(enabled));
                            }}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('show_search_box')}
                        hint={t('show_search_box_hint')}
                        hintKey="show_search_box"
                    />
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={showSearchBox}
                            onChange={(e) => {
                                const enabled = e.target.checked;
                                setShowSearchBox(enabled);
                                saveAppSetting('show_search_box', String(enabled));
                            }}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('scroll_top_button')}
                        hint={t('scroll_top_button_hint')}
                        hintKey="scroll_top_button"
                    />
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={scrollTopButtonEnabled}
                            onChange={(e) => {
                                const enabled = e.target.checked;
                                setScrollTopButtonEnabled(enabled);
                                saveAppSetting('show_scroll_top_button', String(enabled));
                            }}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('emoji_panel_enabled') || '表情包开关'}
                        hint={t('emoji_panel_enabled_hint') || '关闭后隐藏表情包入口'}
                        hintKey="emoji_panel_enabled"
                    />
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={emojiPanelEnabled}
                            onChange={(e) => {
                                const enabled = e.target.checked;
                                setEmojiPanelEnabled(enabled);
                                saveAppSetting('emoji_panel_enabled', String(enabled));
                            }}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('tag_manager_enabled') || '标签管理页开关'}
                        hint={t('tag_manager_enabled_hint') || '关闭后隐藏标签管理入口'}
                        hintKey="tag_manager_enabled"
                    />
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={tagManagerEnabled}
                            onChange={(e) => {
                                const enabled = e.target.checked;
                                setTagManagerEnabled(enabled);
                                saveAppSetting('tag_manager_enabled', String(enabled));
                            }}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('arrow_key_selection')}
                        hint={t('arrow_key_selection_hint')}
                        hintKey="arrow_key_selection"
                    />
                    <label className="switch">
                        <input
                            className="cb"
                            type="checkbox"
                            checked={arrowKeySelection}
                            onChange={(e) => {
                                const enabled = e.target.checked;
                                setArrowKeySelection(enabled);
                                saveAppSetting('arrow_key_selection', String(enabled));
                            }}
                        />
                        <div className="toggle"><div className="left" /><div className="right" /></div>
                    </label>
                </div>

                {/* 卡片密度三档（V5 / 需求 32.1）：紧凑 / 标准 / 宽松，存 app.card_density，默认 standard */}
                <div className="setting-item column">
                    <div className="item-label-group" style={{ marginBottom: '8px' }}>
                        <LabelWithHint
                            label={t('card_density') || '卡片密度'}
                            hint={t('card_density_hint') || '调整剪贴板列表条目的高度与间距，紧凑可一屏看到更多，宽松更舒适'}
                            hintKey="card_density"
                        />
                    </div>
                    <div className="settings-inline-choice-row">
                        {([
                            { id: 'compact', name: t('card_density_compact') || '紧凑' },
                            { id: 'standard', name: t('card_density_standard') || '标准' },
                            { id: 'loose', name: t('card_density_loose') || '宽松' }
                        ] as { id: CardDensity; name: string }[]).map((densityItem) => (
                            <button
                                key={densityItem.id}
                                type="button"
                                onClick={() => {
                                    setCardDensity(densityItem.id);
                                    saveAppSetting('card_density', densityItem.id);
                                }}
                                className={`btn-icon settings-inline-choice-btn ${cardDensity === densityItem.id ? 'active' : ''}`}
                            >
                                {densityItem.name}
                            </button>
                        ))}
                    </div>
                </div>

                {/* macOS cleanup: Removed Restart as Admin */}

                {/* 复制诊断信息（A9 / 需求 7）：收集脱敏后的日志与系统信息写入剪贴板，便于提交反馈 */}
                <div className="setting-item">
                    <LabelWithHint
                        label={t('copy_diagnostics') || '复制诊断信息'}
                        hint={t('copy_diagnostics_hint') || '收集日志末尾、系统信息与设置摘要（已脱敏）到剪贴板，便于提交反馈，不会上传任何数据'}
                        hintKey="copy_diagnostics"
                    />
                    <button
                        type="button"
                        className="btn-icon"
                        style={{ width: 'auto', fontSize: '11px', height: '28px', padding: '0 12px', flexShrink: 0, display: 'inline-flex', alignItems: 'center', gap: '6px' }}
                        onClick={handleCopyDiagnostics}
                    >
                        <ClipboardCopy size={14} />
                        <span>
                            {diagState === 'copied'
                                ? (t('copied') || '已复制')
                                : diagState === 'error'
                                    ? (t('copy_failed') || '复制失败')
                                    : (t('copy_diagnostics') || '复制诊断信息')}
                        </span>
                    </button>
                </div>
            </div>
        )}
    </div>
    );
};

export default GeneralSettingsGroup;
