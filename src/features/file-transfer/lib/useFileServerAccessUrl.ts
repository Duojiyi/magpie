import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "../../../shared/lib/tauriRuntime";

/**
 * The LAN link/QR for the file-transfer page, including the access key. The server rejects
 * requests without it; opening this link once stores the key in a cookie on the phone.
 */
export const useFileServerAccessUrl = (ip: string, port: string | number): string => {
  const [key, setKey] = useState("");

  useEffect(() => {
    if (!isTauriRuntime()) return;
    invoke<string>("get_file_server_access_key").then(setKey).catch(console.error);
  }, []);

  if (!ip || !port) return "";
  return `http://${ip}:${port}/${key ? `?k=${key}` : ""}`;
};
