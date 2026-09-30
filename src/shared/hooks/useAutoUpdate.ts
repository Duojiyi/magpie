import { useState, useEffect, useCallback } from "react";
import { check, Update } from "@tauri-apps/plugin-updater";
import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { isTauriRuntime } from "../lib/tauriRuntime";

export type UpdateStatus = "idle" | "checking" | "downloading" | "ready" | "error";

// 更新检查错误分类对应的 i18n 文案键（DNS / TLS / 通用三类）
export type UpdateErrorKey = "update_error_dns" | "update_error_tls" | "update_error_generic";

/**
 * 将更新检查抛出的原始（英文）异常归类为三类中文文案键，
 * 避免把原始英文异常直接暴露给用户（需求 3.1 / 3.2 / 3.3）。
 * - DNS 解析失败 -> update_error_dns
 * - TLS 握手 / 证书失败 -> update_error_tls
 * - 其他未知错误 -> update_error_generic
 */
export const classifyUpdateError = (raw: string): UpdateErrorKey => {
  const text = raw.toLowerCase();
  // DNS 解析问题：域名无法解析 / 查找失败
  if (text.includes("dns") || text.includes("resolve") || text.includes("failed to lookup")) {
    return "update_error_dns";
  }
  // TLS / 证书 / 握手失败：安全连接建立失败
  if (text.includes("tls") || text.includes("handshake") || text.includes("certificate")) {
    return "update_error_tls";
  }
  return "update_error_generic";
};

// 启动后多久触发首次自动检查更新（毫秒）
const STARTUP_CHECK_DELAY_MS = 5000;

const PORTABLE_RELEASES_URL = "https://github.com/Duojiyi/magpie/releases/latest";

export const useAutoUpdate = () => {
  const [isOpen, setIsOpen] = useState(false);
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [version, setVersion] = useState("");
  const [notes, setNotes] = useState("");
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [updateObj, setUpdateObj] = useState<Update | null>(null);
  // 更新失败时的中文错误分类文案键，不暴露原始英文异常
  const [errorKey, setErrorKey] = useState<UpdateErrorKey>("update_error_generic");

  const checkUpdate = useCallback(async () => {
    if (!isTauriRuntime()) return;
    
    try {
      setStatus("checking");
      
      const update = await check({
        proxy: undefined,
        headers: { "Cache-Control": "no-cache" },
        timeout: 10000
      });

      if (update) {
        console.log(`[Update] New version detected: ${update.version}`);
        setUpdateObj(update);
        setVersion(update.version);
        setNotes(update.body || "");
        setIsOpen(true);
      } else {
        // No update found, emit an event so the UI can show "Up to date"
        import('@tauri-apps/api/event').then(({ emit }) => {
          emit("update-not-available");
        });
      }
      
      setStatus("idle");
    } catch (error) {
      // 仅向控制台输出原始异常用于排查；界面只呈现中文分类文案
      console.error("[Update] Failed to check for updates:", error);
      setErrorKey(classifyUpdateError(String(error)));
      setStatus("error");
    }
  }, []);

  const startUpdate = async () => {
    if (!updateObj) return;

    // Portable build: downloadAndInstall runs the NSIS/MSI installer, which installs a
    // separate copy using %APPDATA% (the portable history seems gone) while the portable exe
    // stays old. Send the user to the release page to grab the new portable zip instead.
    const portable = await invoke<boolean>("is_portable_mode").catch(() => false);
    if (portable) {
      setIsOpen(false);
      invoke("open_content", {
        id: 0,
        content: PORTABLE_RELEASES_URL,
        contentType: "url"
      }).catch(console.error);
      return;
    }

    try {
      setStatus("downloading");
      setDownloadProgress(0);

      await updateObj.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            console.log("[Update] Download started");
            break;
          case "Progress":
            setDownloadProgress((prev) => Math.min(prev + 5, 99));
            break;
          case "Finished":
            console.log("[Update] Download finished");
            setDownloadProgress(100);
            setStatus("ready");
            break;
        }
      });
      
      setStatus("ready");
      setDownloadProgress(100);
    } catch (error) {
      // 仅向控制台输出原始异常用于排查；界面只呈现中文分类文案
      console.error("[Update] Failed to download or install update:", error);
      setErrorKey(classifyUpdateError(String(error)));
      setStatus("error");
    }
  };

  const applyUpdate = async () => {
    try {
      await relaunch();
    } catch (error) {
      console.error("[Update] Failed to relaunch:", error);
    }
  };

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    let cancelled = false;

    // 读取设置：默认开启启动检查（与历史行为一致）
    const scheduleStartupCheck = async () => {
      if (!isTauriRuntime()) return;

      try {
        const settings = await invoke<Record<string, string>>("get_settings");
        if (cancelled) return;

        // app.check_update_on_startup === "false" 时关闭，其它（"true" / undefined）都视为开启
        const enabled = settings["app.check_update_on_startup"] !== "false";
        if (!enabled) return;
      } catch {
        // 读不到设置时按历史行为继续检查
      }

      timer = setTimeout(() => {
        checkUpdate();
      }, STARTUP_CHECK_DELAY_MS);
    };

    scheduleStartupCheck();

    const setupListener = async () => {
      if (isTauriRuntime()) {
        const { listen } = await import('@tauri-apps/api/event');
        return listen("check-update-manually", () => {
          checkUpdate();
        });
      }
      return () => {};
    };

    let unlisten: (() => void) | undefined;
    setupListener().then(fn => { unlisten = fn; });
    
    return () => {
      cancelled = true;
      if (timer) clearTimeout(timer);
      if (unlisten) unlisten();
    };
  }, [checkUpdate]);

  return {
    isOpen,
    status,
    version,
    notes,
    downloadProgress,
    errorKey,
    onManualUpdate: checkUpdate,
    onStartDownload: startUpdate,
    onApplyUpdate: applyUpdate,
    onClose: () => setIsOpen(false),
  };
};
