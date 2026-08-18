// 屏幕键盘用的文本输入辅助：原生 setter 写入 + 派发 input 事件，
// 保证 Svelte 5 bind:value / 原生监听都能感知到变更。

export type TextTargetLike = HTMLInputElement | HTMLTextAreaElement;

export function isTextEntryTarget(element: Element | null): element is TextTargetLike {
  if (!element) return false;
  if (element instanceof HTMLTextAreaElement) {
    return !element.readOnly && !element.disabled;
  }
  if (element instanceof HTMLInputElement) {
    const type = (element.type || "text").toLowerCase();
    const editable = ["text", "search", "url", "tel", "email", "password", "number", ""].includes(type);
    return editable && !element.readOnly && !element.disabled;
  }
  return false;
}

function writeValue(target: TextTargetLike, value: string): void {
  const proto = target instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(proto, "value")?.set;
  if (setter) setter.call(target, value);
  else target.value = value;
  target.dispatchEvent(new Event("input", { bubbles: true }));
}

function selectionRange(target: TextTargetLike): { start: number; end: number } {
  const value = target.value ?? "";
  const start = typeof target.selectionStart === "number" ? target.selectionStart : value.length;
  const end = typeof target.selectionEnd === "number" ? target.selectionEnd : value.length;
  return { start: Math.max(0, start), end: Math.max(0, end) };
}

function placeCaret(target: TextTargetLike, caret: number): void {
  try {
    target.setSelectionRange?.(caret, caret);
  } catch {
    // 某些宿主/元素类型不支持 setSelectionRange，忽略即可
  }
}

/** 在光标处插入文本（替换选区），返回新值 */
export function insertTextAtCursor(target: TextTargetLike, text: string): string {
  const value = target.value ?? "";
  const { start, end } = selectionRange(target);
  const next = value.slice(0, start) + text + value.slice(end);
  writeValue(target, next);
  placeCaret(target, start + text.length);
  return next;
}

/** 退格：有选区删除选区，否则删除光标前一个字符，返回新值 */
export function backspaceAtCursor(target: TextTargetLike): string {
  const value = target.value ?? "";
  const { start, end } = selectionRange(target);
  let next = value;
  let caret = start;
  if (start !== end) {
    next = value.slice(0, start) + value.slice(end);
  } else if (start > 0) {
    next = value.slice(0, start - 1) + value.slice(start);
    caret = start - 1;
  }
  if (next !== value) writeValue(target, next);
  placeCaret(target, caret);
  return next;
}
