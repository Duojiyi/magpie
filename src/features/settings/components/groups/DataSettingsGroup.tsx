import { open, ask, message } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, ChevronRight, Database } from "lucide-react";

interface DataSettingsGroupProps {
    t: (key: string) => string;
    collapsed: boolean;
    onToggle: () => void;
    dataPath: string;
}

const DataSettingsGroup = ({ t, collapsed, onToggle, dataPath }: DataSettingsGroupProps) => (
    <div className={`settings-group ${collapsed ? 'collapsed' : ''}`}>
        <div className="group-header" onClick={onToggle}>
            {/* 标题区统一使用 lucide 图标（需求 30.1/30.2） */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Database size={16} />
                <h3 style={{ margin: 0 }}>{t('data_management')}</h3>
            </div>
            {collapsed ? <ChevronRight size={16} /> : <ChevronDown size={16} />}
        </div>
        {!collapsed && (
            <div className="group-content">
                <div className="setting-item column no-border">
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                        <span className="item-label" style={{ textTransform: 'uppercase', fontSize: '11px', opacity: 0.8 }}>{t('data_path')}</span>
                        <div style={{ display: 'flex', gap: '8px' }}>
                            <button
                                className="btn-icon"
                                onClick={() => {
                                    open({
                                        directory: true,
                                        multiple: false,
                                        title: t('change_data_path')
                                    }).then(async (selected) => {
                                        if (selected) {
                                            const newPath = selected as string;
                                            const confirm = await ask(
                                                t('data_move_confirm').replace('{path}', newPath),
                                                { title: t('change_data_path'), kind: 'warning', okLabel: t('confirm'), cancelLabel: t('cancel') }
                                            );

                                            if (confirm) {
                                                try {
                                                    // set_data_path copies the data and points datapath.txt at the
                                                    // new folder; on success it also pauses capture until restart.
                                                    await invoke("set_data_path", { newPath });

                                                    // Committed: relaunch no matter how the notice resolves, or
                                                    // capture would stay paused with a misleading "failed" dialog.
                                                    try {
                                                        await message(
                                                            t('data_move_success'),
                                                            { title: t('notice'), kind: 'info' }
                                                        );
                                                    } finally {
                                                        await invoke("relaunch");
                                                    }
                                                } catch (e: unknown) {
                                                    console.error(e);
                                                    const errorMsg = e instanceof Error ? e.message : String(e);
                                                    await message(
                                                        t('data_move_failed').replace('{e}', errorMsg),
                                                        { title: t('error'), kind: 'error' }
                                                    );
                                                }
                                            }
                                        }
                                    });
                                }}
                                style={{ width: 'auto', padding: '4px 12px', fontSize: '10px', textTransform: 'uppercase', height: '24px' }}
                            >
                                {t('change_app')}
                            </button>
                            <button
                                className="btn-icon"
                                onClick={() => invoke("open_data_folder").catch(console.error)}
                                title={t('open_folder') || "Open"}
                                style={{ width: 'auto', padding: '4px 12px', fontSize: '10px', textTransform: 'uppercase', height: '24px' }}
                            >
                                {t('open_folder')}
                            </button>
                        </div>
                    </div>
                    <div className="data-panel" style={{ fontSize: '11px', color: 'var(--text-secondary)', wordBreak: 'break-all' }}>
                        {dataPath}
                    </div>
                </div>
            </div>
        )}
    </div>
);

export default DataSettingsGroup;
