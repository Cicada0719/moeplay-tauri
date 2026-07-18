import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  activateGamepadFocus,
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

  it("focuses the route search field", () => {
    const search = document.createElement("input");
    search.type = "search";
    document.body.append(search);
    place(search, 0, 0);
    expect(focusGamepadSearch()).toBe(search);
    expect(document.activeElement).toBe(search);
  });
});
