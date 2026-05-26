import type { StoreTheme, ThemeListResponse, ThemeSort } from "./types";

// 主题商店后端 API。默认未配置时所有请求会抛出 not_configured 错误，
// theme-store 面板将显示空列表/失败提示，不向上游服务器发请求。
// 如需启用：构建时设置 VITE_API_BASE_URL，或自行部署兼容服务。
const API_BASE = import.meta.env.VITE_API_BASE_URL || "";

function ensureConfigured() {
  if (!API_BASE) {
    throw new Error("theme_store_not_configured");
  }
}

function getToken(): string | null {
  return localStorage.getItem("tiez_theme_store_token");
}

function authHeaders(): Record<string, string> {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

export async function register(
  username: string,
  password: string
): Promise<{ token: string; username: string }> {
  ensureConfigured();
  const res = await fetch(`${API_BASE}/api/v1/auth/register`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
    signal: AbortSignal.timeout(10000),
  });
  if (!res.ok) {
    const data = await res.json().catch(() => ({}));
    throw new Error(data.error || "Registration failed");
  }
  return res.json();
}

export async function login(
  username: string,
  password: string
): Promise<{ token: string; username: string }> {
  ensureConfigured();
  const res = await fetch(`${API_BASE}/api/v1/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
    signal: AbortSignal.timeout(10000),
  });
  if (!res.ok) {
    const data = await res.json().catch(() => ({}));
    throw new Error(data.error || "Login failed");
  }
  return res.json();
}

export async function fetchThemes(params: {
  page?: number;
  limit?: number;
  sort?: ThemeSort;
  category?: string;
  q?: string;
  token?: string;
}): Promise<ThemeListResponse> {
  ensureConfigured();
  const searchParams = new URLSearchParams();
  if (params.page) searchParams.set("page", String(params.page));
  if (params.limit) searchParams.set("limit", String(params.limit));
  if (params.sort) searchParams.set("sort", params.sort);
  if (params.category) searchParams.set("category", params.category);
  if (params.q) searchParams.set("q", params.q);

  const res = await fetch(
    `${API_BASE}/api/v1/themes?${searchParams.toString()}`,
    {
      headers: authHeaders(),
      signal: AbortSignal.timeout(10000),
    }
  );
  if (!res.ok) throw new Error("Failed to fetch themes");
  return res.json();
}

export async function fetchThemeDetail(id: string): Promise<StoreTheme> {
  ensureConfigured();
  const res = await fetch(`${API_BASE}/api/v1/themes/${id}`, {
    headers: authHeaders(),
    signal: AbortSignal.timeout(10000),
  });
  if (!res.ok) throw new Error("Failed to fetch theme detail");
  return res.json();
}

export async function fetchThemeCSS(id: string): Promise<string> {
  ensureConfigured();
  const res = await fetch(`${API_BASE}/api/v1/themes/${id}/css`, {
    signal: AbortSignal.timeout(10000),
  });
  if (!res.ok) throw new Error("Failed to fetch theme CSS");
  return res.text();
}

export function getPreviewUrl(id: string, type: "light" | "dark"): string {
  if (!API_BASE) return "";
  return `${API_BASE}/api/v1/themes/${id}/preview/${type}`;
}

export async function uploadTheme(file: File): Promise<StoreTheme> {
  ensureConfigured();
  const formData = new FormData();
  formData.append("file", file);

  const res = await fetch(`${API_BASE}/api/v1/themes`, {
    method: "POST",
    headers: authHeaders(),
    body: formData,
    signal: AbortSignal.timeout(30000),
  });
  if (!res.ok) {
    const data = await res.json().catch(() => ({}));
    throw new Error(data.error || "Upload failed");
  }
  return res.json();
}

export async function rateTheme(
  id: string,
  score: number
): Promise<{ avgRating: number; ratingCount: number }> {
  ensureConfigured();
  const res = await fetch(`${API_BASE}/api/v1/themes/${id}/rate`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...authHeaders() },
    body: JSON.stringify({ score }),
    signal: AbortSignal.timeout(10000),
  });
  if (!res.ok) {
    const data = await res.json().catch(() => ({}));
    throw new Error(data.error || "Rating failed");
  }
  return res.json();
}

export async function deleteTheme(id: string): Promise<void> {
  ensureConfigured();
  const res = await fetch(`${API_BASE}/api/v1/themes/${id}`, {
    method: "DELETE",
    headers: authHeaders(),
    signal: AbortSignal.timeout(10000),
  });
  if (!res.ok) {
    const data = await res.json().catch(() => ({}));
    throw new Error(data.error || "Delete failed");
  }
}
