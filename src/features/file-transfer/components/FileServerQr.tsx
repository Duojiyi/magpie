import { QRCodeCanvas } from "qrcode.react";
import { useFileServerAccessUrl } from "../lib/useFileServerAccessUrl";

/**
 * QR code for the LAN file-transfer page; carries the access key the server requires.
 * Clicking it copies the same link, for devices that cannot scan (a second PC's browser).
 */
const FileServerQr = ({ ip, port, size }: { ip: string; port: string | number; size: number }) => {
  const url = useFileServerAccessUrl(ip, port);
  if (!url) return null;
  return (
    <button
      type="button"
      title={url}
      aria-label={url}
      onClick={() => navigator.clipboard?.writeText(url).catch(console.error)}
      style={{ padding: 0, border: "none", background: "none", cursor: "copy", lineHeight: 0 }}
    >
      <QRCodeCanvas value={url} size={size} />
    </button>
  );
};

export default FileServerQr;
