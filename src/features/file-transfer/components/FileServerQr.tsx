import { QRCodeCanvas } from "qrcode.react";
import { useFileServerAccessUrl } from "../lib/useFileServerAccessUrl";

/** QR code for the LAN file-transfer page; carries the access key the server requires. */
const FileServerQr = ({ ip, port, size }: { ip: string; port: string | number; size: number }) => {
  const url = useFileServerAccessUrl(ip, port);
  if (!url) return null;
  return <QRCodeCanvas value={url} size={size} />;
};

export default FileServerQr;
