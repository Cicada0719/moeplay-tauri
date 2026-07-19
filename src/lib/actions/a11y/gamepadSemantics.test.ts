import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  adjustFocusedGamepadControl,
  gamepadElementLabel,
  gamepadPrimaryActionLabel,
  gamepadSecondaryActionLabel,
} from "./gamepadSemantics";

beforeEach(() => {
  document.body.innerHTML = "";
});

describe("gamepad control semantics", () => {
  it("derives contextual labels and primary actions from accessible controls", () => {
    const button = document.createElement("button");
    button.setAttribute("aria-label", "打开 星海回声 档案");
    document.body.append(button);
    expect(gamepadElementLabel(button)).toBe("打开 星海回声 档案");
    expect(gamepadPrimaryActionLabel(button)).toBe("打开");

    button.dataset.gamepadActivate = "启动游戏";
    expect(gamepadPrimaryActionLabel(button)).toBe("启动游戏");
  });

  it("finds a card-level Y secondary action", () => {
    const group = document.createElement("article");
    group.dataset.gamepadGroup = "";
    const primary = document.createElement("button");
    const secondary = document.createElement("button");
    secondary.dataset.gamepadSecondaryAction = "";
    secondary.dataset.gamepadActivate = "收藏";
    group.append(primary, secondary);
    document.body.append(group);
    expect(gamepadSecondaryActionLabel(primary)).toBe("收藏");
  });

  it("uses left and right to adjust focused ranges", () => {
    const input = document.createElement("input");
    input.type = "range";
    input.min = "0";
    input.max = "10";
    input.step = "2";
    input.value = "4";
    const onInput = vi.fn();
    const onChange = vi.fn();
    input.addEventListener("input", onInput);
    input.addEventListener("change", onChange);
    document.body.append(input);
    input.focus();

    expect(adjustFocusedGamepadControl("right")).toBe(true);
    expect(input.value).toBe("6");
    expect(onInput).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledOnce();
    expect(adjustFocusedGamepadControl("down")).toBe(false);
  });

  it("uses left and right to adjust focused selects", () => {
    const select = document.createElement("select");
    for (const [label, value] of [["一", "1"], ["二", "2"], ["三", "3"]]) {
      const option = document.createElement("option");
      option.textContent = label;
      option.value = value;
      select.append(option);
    }
    select.selectedIndex = 1;
    const onChange = vi.fn();
    select.addEventListener("change", onChange);
    document.body.append(select);
    select.focus();

    expect(adjustFocusedGamepadControl("left")).toBe(true);
    expect(select.value).toBe("1");
    expect(onChange).toHaveBeenCalledOnce();
  });
});
