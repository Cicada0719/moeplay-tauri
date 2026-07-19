import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  activateGamepadFocus,
  activateGamepadSecondaryFocus,
  collectGamepadFocusable,
  focusGamepadSearch,
  moveGamepadFocus,
} from "./domGamepadNavigation";

function place(element: HTMLElement, left: number, top: number, width = 100, height = 40) {
  element.getBoundingClientRect = () => ({
    x: left, y: top, left, top, width, height,
    right: left + width, bottom: top + height,
    toJSON: () => ({}),
  } as DOMRect);
}

beforeEach(() => {
  document.body.innerHTML = "";
  vi.spyOn(HTMLElement.prototype, "scrollIntoView").mockImplementation(() => {});
});

describe("DOM gamepad spatial navigation", () => {
  it("moves by visual direction instead of DOM order", () => {
    const center = document.createElement("button");
    const right = document.createElement("button");
    const down = document.createElement("button");
    document.body.append(down, center, right);
    place(center, 100, 100);
    place(right, 240, 100);
    place(down, 100, 220);
    center.focus();

    expect(moveGamepadFocus("right")).toBe(right);
    expect(document.activeElement).toBe(right);
    center.focus();
    expect(moveGamepadFocus("down")).toBe(down);
  });

  it("ignores hidden, disabled and inert controls", () => {
    const visible = document.createElement("button");
    const disabled = document.createElement("button");
    disabled.disabled = true;
    const hidden = document.createElement("button");
    hidden.hidden = true;
    const inertHost = document.createElement("div");
    inertHost.setAttribute("inert", "");
    const inertButton = document.createElement("button");
    inertHost.append(inertButton);
    document.body.append(visible, disabled, hidden, inertHost);
    place(visible, 0, 0);
    place(disabled, 0, 50);
    place(hidden, 0, 100);
    place(inertButton, 0, 150);
    expect(collectGamepadFocusable()).toEqual([visible]);
  });

  it("focuses first control before activation, then clicks it", () => {
    const button = document.createElement("button");
    const click = vi.fn();
    button.addEventListener("click", click);
    document.body.append(button);
    place(button, 0, 0);

    expect(activateGamepadFocus()).toBe(button);
    expect(click).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(button);
    activateGamepadFocus();
    expect(click).toHaveBeenCalledOnce();
  });

  it("includes role-based internal controls and skips explicit controller exclusions", () => {
    const roleButton = document.createElement("div");
    roleButton.setAttribute("role", "button");
    roleButton.tabIndex = 0;
    const skipped = document.createElement("button");
    skipped.dataset.gamepadSkip = "true";
    document.body.append(roleButton, skipped);
    place(roleButton, 0, 0);
    place(skipped, 0, 50);
    expect(collectGamepadFocusable()).toEqual([roleButton]);
  });

  it("keeps roving tabs controller-reachable while excluding generic negative tab stops", () => {
    const rovingTab = document.createElement("button");
    rovingTab.setAttribute("role", "tab");
    rovingTab.tabIndex = -1;
    const backdrop = document.createElement("button");
    backdrop.tabIndex = -1;
    document.body.append(rovingTab, backdrop);
    place(rovingTab, 0, 0);
    place(backdrop, 0, 50);

    expect(collectGamepadFocusable()).toEqual([rovingTab]);
  });

  it("runs the focused card secondary action with Y semantics", () => {
    const group = document.createElement("article");
    group.dataset.gamepadGroup = "";
    const primary = document.createElement("button");
    const secondary = document.createElement("button");
    secondary.dataset.gamepadSecondaryAction = "";
    const click = vi.fn();
    secondary.addEventListener("click", click);
    group.append(primary, secondary);
    document.body.append(group);
    place(primary, 0, 0);
    place(secondary, 0, 50);
    primary.focus();
    expect(activateGamepadSecondaryFocus()).toBe(secondary);
    expect(click).toHaveBeenCalledOnce();
  });

  it("moves through roving tablists before applying geometric navigation", () => {
    const tabList = document.createElement("div");
    tabList.setAttribute("role", "tablist");
    const first = document.createElement("button");
    first.setAttribute("role", "tab");
    first.tabIndex = 0;
    const second = document.createElement("button");
    second.setAttribute("role", "tab");
    second.tabIndex = -1;
    const unrelated = document.createElement("button");
    tabList.append(first, second);
    document.body.append(tabList, unrelated);
    place(first, 0, 100);
    place(second, 120, 100);
    place(unrelated, 50, 0);
    first.focus();

    expect(moveGamepadFocus("right")).toBe(second);
    expect(document.activeElement).toBe(second);
  });

  it("honors explicit directional overrides for dense internal layouts", () => {
    const search = document.createElement("input");
    search.type = "search";
    search.dataset.gamepadNavRight = "#source-auto";
    const source = document.createElement("button");
    source.id = "source-auto";
    document.body.append(search, source);
    place(search, 0, 0, 500, 40);
    place(source, 600, 0, 80, 40);
    search.focus();

    expect(moveGamepadFocus("right")).toBe(source);
    expect(document.activeElement).toBe(source);
  });

  it("stays on the current control at a visual edge", () => {
    const left = document.createElement("button");
    const right = document.createElement("button");
    document.body.append(left, right);
    place(left, 0, 0);
    place(right, 120, 0);
    right.focus();
    expect(moveGamepadFocus("right")).toBe(right);
    expect(document.activeElement).toBe(right);
  });

  it("focuses the first visible route search field", () => {
    const staleSearch = document.createElement("input");
    staleSearch.type = "search";
    staleSearch.hidden = true;
    const search = document.createElement("input");
    search.type = "search";
    document.body.append(staleSearch, search);
    place(staleSearch, 0, 0);
    place(search, 0, 50);
    expect(focusGamepadSearch()).toBe(search);
    expect(document.activeElement).toBe(search);
  });
});
