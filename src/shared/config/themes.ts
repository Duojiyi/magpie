import type { Locale } from "../types";

/** 新主题标识：墨玉 / 宣纸 / 晨雾 / 暮山 */
export type ThemeId = "ink" | "paper" | "mist" | "dusk";

/** 主题定义模型（见设计文档 §2.1） */
export interface ThemeDefinition {
  /** 主题标识 */
  id: ThemeId;
  /** 各语言显示名 */
  labels: Record<Locale, string>;
  /** 是否液态玻璃主题（仅 mist/dusk 为 true） */
  glass: boolean;
  /** 是否支持自定义背景（仅玻璃主题为 true） */
  supportsCustomBackground?: boolean;
  /** 是否支持表面透明度（仅玻璃主题为 true） */
  supportsSurfaceOpacity?: boolean;
}

/** 默认主题：前后端统一兜底为墨玉 ink（见设计文档 §2.1、Requirement 2.1） */
export const DEFAULT_THEME: ThemeId = "ink";

/**
 * 主题列表，ink 居首。
 * 玻璃主题（mist/dusk）三项能力元数据为 true，扁平主题（ink/paper）省略即为 false。
 */
export const THEMES: ThemeDefinition[] = [
  {
    id: "ink",
    labels: {
      zh: "墨玉",
      en: "Ink Jade",
      tw: "墨玉"
    },
    glass: false
  },
  {
    id: "paper",
    labels: {
      zh: "宣纸",
      en: "Paper",
      tw: "宣紙"
    },
    glass: false
  },
  {
    id: "mist",
    labels: {
      zh: "晨雾",
      en: "Mist",
      tw: "晨霧"
    },
    glass: true,
    supportsCustomBackground: true,
    supportsSurfaceOpacity: true
  },
  {
    id: "dusk",
    labels: {
      zh: "暮山",
      en: "Dusk",
      tw: "暮山"
    },
    glass: true,
    supportsCustomBackground: true,
    supportsSurfaceOpacity: true
  }
];

/**
 * 旧主题别名表（权威见设计文档表 4.1）。
 * 精确字符串匹配（区分大小写）：旧主题值 → 新主题值。
 * 注意：store-* 前缀、空串、未知值等不在此表中，由 normalizeThemeId 兜底为 ink。
 */
export const LEGACY_THEME_MAP: Record<string, ThemeId> = {
  mica: "mist",
  acrylic: "dusk",
  sakura: "mist",
  retro: "ink",
  "sticky-note": "paper",
  paper: "paper"
};

const THEME_BY_ID = new Map<ThemeId, ThemeDefinition>(
  THEMES.map((theme) => [theme.id, theme])
);

const NEW_THEME_IDS = new Set<string>(THEMES.map((theme) => theme.id));

/** 判断是否为已注册的新主题标识 */
const isThemeId = (value: string): value is ThemeId => NEW_THEME_IDS.has(value);

/**
 * 主题归一器：将任意输入映射为 {ink, paper, mist, dusk} 中恰好一个值。
 * 规则（见 Requirement 3.1~3.5）：
 * - 已是新主题值 → 原样返回（满足幂等）
 * - 命中旧别名表 → 精确映射为新值
 * - store-* 前缀 / 空串 / 纯空白 / null / 非字符串 / 未知值 → ink
 */
export const normalizeThemeId = (themeId: unknown): ThemeId => {
  // 非字符串（含 null/undefined/数字等）一律落默认
  if (typeof themeId !== "string") return DEFAULT_THEME;
  // 已是新主题值，原样返回
  if (isThemeId(themeId)) return themeId;
  // 旧别名精确匹配（区分大小写）；仅匹配自有属性，避免命中 Object 原型链上的同名键
  // （如字符串 "valueOf"/"toString"/"constructor" 会经原型链解析为函数而误判）
  if (Object.prototype.hasOwnProperty.call(LEGACY_THEME_MAP, themeId)) {
    return LEGACY_THEME_MAP[themeId];
  }
  // store-* / 空串 / 纯空白 / 未知值统一落默认
  return DEFAULT_THEME;
};

/**
 * 取主题定义：先归一再查表，归一后必命中，保底返回默认主题定义。
 */
export const getThemeDefinition = (themeId: unknown): ThemeDefinition =>
  THEME_BY_ID.get(normalizeThemeId(themeId)) ?? THEME_BY_ID.get(DEFAULT_THEME)!;

/**
 * 取主题显示名：先归一再取对应语言标签。
 */
export const getThemeLabel = (themeId: unknown, locale: Locale): string =>
  getThemeDefinition(themeId).labels[locale];

/**
 * 玻璃判定器：先归一再判定，仅 mist/dusk 返回 true。
 * 异常输入（空串/null/非字符串/未知值）经归一落默认 ink，返回 false，不抛错。
 */
export const isGlassTheme = (themeId: unknown): boolean =>
  getThemeDefinition(themeId).glass;

/**
 * 是否支持自定义背景：仅玻璃主题为 true（与 isGlassTheme 收敛一致，见 Property 5）。
 */
export const supportsCustomBackground = (themeId: unknown): boolean =>
  Boolean(getThemeDefinition(themeId).supportsCustomBackground);

/**
 * 是否支持表面透明度：仅玻璃主题为 true（与 isGlassTheme 收敛一致，见 Property 5）。
 */
export const supportsSurfaceOpacity = (themeId: unknown): boolean =>
  Boolean(getThemeDefinition(themeId).supportsSurfaceOpacity);

/**
 * 清理目标根元素上全部以 `theme-` 为前缀的 class（含所有 theme-{id} 与 theme-glass）。
 */
export const clearThemeClasses = (
  ...targets: Array<Element | null | undefined>
) => {
  for (const target of targets) {
    if (!target) continue;
    const toRemove = [...target.classList].filter((c) => c.startsWith("theme-"));
    if (toRemove.length > 0) target.classList.remove(...toRemove);
  }
};

/**
 * 主题应用器：归一后先清理全部 theme- class，再添加 theme-{id}；
 * isGlassTheme 为真时追加 theme-glass。保证目标元素恰好保留一个 theme-{id}。
 */
export const applyThemeClasses = (
  themeId: unknown,
  ...targets: Array<Element | null | undefined>
) => {
  const id = normalizeThemeId(themeId);
  const glass = isGlassTheme(id);
  clearThemeClasses(...targets);
  for (const target of targets) {
    if (!target) continue;
    target.classList.add(`theme-${id}`);
    if (glass) target.classList.add("theme-glass");
  }
};
