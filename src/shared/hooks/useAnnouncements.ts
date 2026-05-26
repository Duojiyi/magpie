import { useState, useEffect } from "react";
import type { Announcement } from "../types";

export type { Announcement } from "../types";

// 公告/心跳服务依赖于上游官网（jimuzhe），fork 版本不再向外发起 ping。
// 保留 hook 接口与 dismiss 行为以兼容现有 UI 调用者。

export function useAnnouncements() {
  const [announcements, setAnnouncements] = useState<Announcement[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setAnnouncements([]);
    setLoading(false);
  }, []);

  const dismissAnnouncement = (id: string, forever: boolean = true) => {
    setAnnouncements((prev) => prev.filter((a) => a.id !== id));
    if (forever) {
      const dismissed = JSON.parse(
        localStorage.getItem("dismissed_announcements") || "[]"
      );
      if (!dismissed.includes(id)) {
        dismissed.push(id);
        localStorage.setItem(
          "dismissed_announcements",
          JSON.stringify(dismissed)
        );
      }
    }
  };

  return { announcements, loading, dismissAnnouncement };
}
