import { describe, expect, it } from "vitest";
import { backspaceAtCursor, insertTextAtCursor, isTextEntryTarget } from "./textInput";

function makeInput(value = ""): HTMLInputElement {
  const input = document.createElement("input");
  input.type = "text";
  input.value = value;
  document.body.appendChild(input);
  return input;
}

describe("textInput 屏幕键盘辅助", () => {
  it("识别可输入目标，排除不可编辑类型", () => {
    const text = makeInput();
    expect(isTextEntryTarget(text)).toBe(true);
    const search = document.createElement("input");
    search.type = "search";
    expect(isTextEntryTarget(search)).toBe(true);
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    expect(isTextEntryTarget(checkbox)).toBe(false);
    const readonly = makeInput();
    readonly.readOnly = true;
    expect(isTextEntryTarget(readonly)).toBe(false);
    const textarea = document.createElement("textarea");
    expect(isTextEntryTarget(textarea)).toBe(true);
    expect(isTextEntryTarget(null)).toBe(false);
  });

  it("在光标处插入并派发 input 事件", () => {
    const input = makeInput("ab");
    let fired = 0;
    input.addEventListener("input", () => { fired += 1; });
    input.setSelectionRange(1, 1);
    expect(insertTextAtCursor(input, "X")).toBe("aXb");
    expect(input.value).toBe("aXb");
    expect(fired).toBe(1);
  });

  it("插入替换选区", () => {
    const input = makeInput("abcd");
    input.setSelectionRange(1, 3);
    expect(insertTextAtCursor(input, "Z")).toBe("aZd");
  });

  it("退格删除光标前字符；有选区时删除选区", () => {
    const input = makeInput("abcd");
    input.setSelectionRange(2, 2);
    expect(backspaceAtCursor(input)).toBe("acd");
    input.setSelectionRange(1, 3);
    expect(backspaceAtCursor(input)).toBe("a");
    input.setSelectionRange(0, 0);
    expect(backspaceAtCursor(input)).toBe("a"); // 光标在开头不动
  });
});
