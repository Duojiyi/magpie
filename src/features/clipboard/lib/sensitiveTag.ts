/**
 * 敏感标记（F3 / 需求 17）相关的纯逻辑。
 *
 * 将「条目是否带敏感标签」「列表视觉强调判定」从 ClipboardItem 组件中抽离为纯函数，
 * 供组件复用并单独测试。新写入统一使用保留标签 __sensitive__，
 * 同时兼容历史数据中的 `sensitive` / `密码` / `password`。
 */

import { SENSITIVE_TAG } from "../../tag/lib/floatingTagLogic";

/** 敏感标签识别集合：保留标签 __sensitive__（F3 写入统一用此值）+ 历史兼容标签。 */
export const SENSITIVE_TAG_NAMES: readonly string[] = [
  SENSITIVE_TAG,
  "sensitive",
  "密码",
  "password",
];

/** 判断条目是否带敏感标签（用于隐私模糊与列表视觉强调，需求 17.1 / 17.2）。 */
export const hasSensitiveTag = (tags?: string[]): boolean =>
  // Case-insensitive, like the backend's has_sensitive_tag: a "Sensitive" tag is encrypted
  // there, so it must be masked here too.
  !!tags?.some((tag) => SENSITIVE_TAG_NAMES.includes(tag.toLowerCase()));

/**
 * 计算条目根容器的 className（需求 17.2）。
 *
 * 带敏感标签的条目追加 `sensitive-item` 类用于视觉强调（色块/边框等）。
 * 该函数集中拼接条目状态类，便于在测试中断言强调样式是否生效。
 */
export function clipboardItemClassName(opts: {
  isSelected: boolean;
  compactMode: boolean;
  isPinned: boolean;
  tags?: string[];
  extraClassName?: string;
}): string {
  const sensitive = hasSensitiveTag(opts.tags);
  return [
    "history-item",
    opts.isSelected ? "selected" : "",
    opts.compactMode ? "compact" : "",
    opts.isPinned ? "pinned" : "",
    sensitive ? "sensitive-item" : "",
    opts.extraClassName || "",
  ]
    .filter(Boolean)
    .join(" ");
}

/** 是否在条目元信息区展示敏感标识图标（ShieldAlert）（需求 17.2）。 */
export const shouldShowSensitiveIcon = (tags?: string[]): boolean =>
  hasSensitiveTag(tags);
