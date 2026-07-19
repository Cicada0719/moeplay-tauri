import type { SpatialDirection } from "./domGamepadNavigation";

function compactText(value: string | null | undefined): string {
  return (value ?? "").replace(/\s+/g, " ").trim();
}

export function gamepadElementLabel(element: HTMLElement | null): string {
  if (!element) return "当前页面";
  const explicit = compactText(element.dataset.gamepadLabel);
  if (explicit) return explicit;
  const aria = compactText(element.getAttribute("aria-label"));
  if (aria) return aria;
  const labelledBy = element.getAttribute("aria-labelledby");
  if (labelledBy && typeof document !== "undefined") {
    const label = compactText(labelledBy.split(/\s+/).map((id) => document.getElementById(id)?.textContent ?? "").join(" "));
    if (label) return label;
  }
  const title = compactText(element.getAttribute("title"));
  if (title) return title;
  const text = compactText(element.textContent);
  if (text) return text.slice(0, 42);
  if (element instanceof HTMLInputElement && element.placeholder) return compactText(element.placeholder);
  return "当前控件";
}

export function gamepadSecondaryActionLabel(element: HTMLElement | null): string | null {
  const group = element?.closest<HTMLElement>("[data-gamepad-group]");
  const secondary = group?.querySelector<HTMLElement>("[data-gamepad-secondary-action]:not([disabled])") ?? null;
  if (!secondary) return null;
  return compactText(secondary.dataset.gamepadActivate)
    || compactText(secondary.dataset.gamepadLabel)
    || compactText(secondary.getAttribute("aria-label"))
    || compactText(secondary.textContent)
    || "次要操作";
}

export function gamepadPrimaryActionLabel(element: HTMLElement | null): string {
  const explicit = compactText(element?.dataset.gamepadActivate);
  if (explicit) return explicit;
  if (!element) return "确认";
  if (element instanceof HTMLInputElement) {
    if (element.type === "checkbox" || element.type === "radio") return element.checked ? "取消选择" : "选择";
    if (element.type === "range") return "调整";
    if (element.type === "search" || element.type === "text") return "输入";
  }
  if (element instanceof HTMLSelectElement) return "选择选项";
  if (element.getAttribute("aria-haspopup") === "dialog") return "打开";
  if (element.getAttribute("aria-expanded") != null) return element.getAttribute("aria-expanded") === "true" ? "收起" : "展开";
  if (element.getAttribute("aria-pressed") != null) return element.getAttribute("aria-pressed") === "true" ? "取消" : "选择";
  const label = gamepadElementLabel(element);
  if (/删除|移除/.test(label)) return "删除";
  if (/返回|关闭|退出/.test(label)) return "返回";
  if (/启动|继续游玩|运行/.test(label)) return "启动";
  if (/播放|继续观看|继续阅读/.test(label)) return "播放";
  if (/打开|查看|进入|详情|档案/.test(label)) return "打开";
  if (/搜索|查找/.test(label)) return "搜索";
  if (/下载|导出/.test(label)) return "下载";
  if (/收藏/.test(label)) return "收藏";
  return "确认";
}

function dispatchValueChange(element: HTMLElement) {
  element.dispatchEvent(new Event("input", { bubbles: true }));
  element.dispatchEvent(new Event("change", { bubbles: true }));
}

export function adjustFocusedGamepadControl(direction: SpatialDirection, element: Element | null = document.activeElement): boolean {
  if (!(element instanceof HTMLElement)) return false;
  if (direction !== "left" && direction !== "right") return false;
  const delta = direction === "right" ? 1 : -1;

  if (element instanceof HTMLInputElement && element.type === "range" && !element.disabled) {
    const parsedMin = Number(element.min);
    const parsedMax = Number(element.max);
    const min = Number.isFinite(parsedMin) ? parsedMin : 0;
    const max = Number.isFinite(parsedMax) ? parsedMax : 100;
    const rawStep = element.step === "any" ? (max - min) / 100 : Number(element.step || 1);
    const step = Number.isFinite(rawStep) && rawStep > 0 ? rawStep : 1;
    const current = Number.isFinite(element.valueAsNumber) ? element.valueAsNumber : Number(element.value || min);
    const next = Math.min(max, Math.max(min, current + delta * step));
    if (next === current) return true;
    element.valueAsNumber = next;
    dispatchValueChange(element);
    return true;
  }

  if (element instanceof HTMLSelectElement && !element.disabled && element.options.length > 0) {
    const next = Math.min(element.options.length - 1, Math.max(0, element.selectedIndex + delta));
    if (next === element.selectedIndex) return true;
    element.selectedIndex = next;
    dispatchValueChange(element);
    return true;
  }

  return false;
}
