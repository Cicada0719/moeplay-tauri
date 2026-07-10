import { afterEach, describe, expect, it, vi } from "vitest";
import { focusTrap, getFocusableElements } from "./focusTrap";

const cleanups: Array<() => void> = [];

function flushMicrotasks() {
  return Promise.resolve().then(() => Promise.resolve());
}

afterEach(() => {
  while (cleanups.length) cleanups.pop()?.();
  document.body.replaceChildren();
});

describe("focusTrap", () => {
  it("filters disabled, hidden, inert and aria-disabled controls", () => {
    const node = document.createElement("div");
    node.innerHTML = `
      <button id="ok">可用</button>
      <button disabled>禁用</button>
      <button hidden>隐藏</button>
      <div inert><button>惰性</button></div>
      <a href="#" aria-disabled="true">不可用链接</a>
    `;
    document.body.append(node);

    expect(getFocusableElements(node).map((item) => item.id)).toEqual(["ok"]);
  });

  it("keeps nested overlays exclusive and restores each trigger in order", async () => {
    const pageTrigger = document.createElement("button");
    const outer = document.createElement("div");
    outer.tabIndex = -1;
    outer.innerHTML = '<button id="outer-trigger">打开内层</button>';
    const inner = document.createElement("div");
    inner.tabIndex = -1;
    inner.innerHTML = '<button id="inner-action">内层操作</button>';
    document.body.append(pageTrigger, outer, inner);

    pageTrigger.focus();
    const outerEscape = vi.fn();
    const outerTrap = focusTrap(outer, { onEscape: outerEscape });
    cleanups.push(() => outerTrap.destroy());
    await flushMicrotasks();

    const outerTrigger = outer.querySelector<HTMLButtonElement>("#outer-trigger")!;
    outerTrigger.focus();
    const innerEscape = vi.fn();
    const innerTrap = focusTrap(inner, { onEscape: innerEscape });
    cleanups.push(() => innerTrap.destroy());
    await flushMicrotasks();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(innerEscape).toHaveBeenCalledOnce();
    expect(outerEscape).not.toHaveBeenCalled();

    innerTrap.destroy();
    cleanups.pop();
    await flushMicrotasks();
    expect(outerTrigger).toHaveFocus();

    outerTrap.destroy();
    cleanups.pop();
    await flushMicrotasks();
    expect(pageTrigger).toHaveFocus();
  });
});
