import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { isTextInput, fieldKeys, onGlobalEscape } from "./keys";

function press(node: Element, key: string, init: KeyboardEventInit = {}) {
  const e = new KeyboardEvent("keydown", {
    key,
    bubbles: true,
    cancelable: true,
    ...init,
  });
  node.dispatchEvent(e);
  return e;
}

describe("isTextInput", () => {
  it("recognizes text-like inputs and textareas", () => {
    for (const type of ["text", "search", "number", "date", "email"]) {
      const el = document.createElement("input");
      el.type = type;
      expect(isTextInput(el)).toBe(true);
    }
    expect(isTextInput(document.createElement("textarea"))).toBe(true);
  });

  it("rejects non-text controls and non-elements", () => {
    const cb = document.createElement("input");
    cb.type = "checkbox";
    expect(isTextInput(cb)).toBe(false);
    expect(isTextInput(document.createElement("button"))).toBe(false);
    expect(isTextInput(null)).toBe(false);
  });
});

describe("fieldKeys", () => {
  let input: HTMLInputElement;
  beforeEach(() => {
    input = document.createElement("input");
    input.type = "text";
    document.body.appendChild(input);
  });
  afterEach(() => input.remove());

  it("ESC reverts to the committed value and blurs, swallowing the event", () => {
    const action = fieldKeys(input, () => "committed");
    input.value = "half-typed";
    input.focus();
    const e = press(input, "Escape");
    expect(input.value).toBe("committed");
    expect(document.activeElement).not.toBe(input);
    expect(e.defaultPrevented).toBe(true);
    // stopPropagation: a window listener must not observe the ESC.
    action.destroy();
  });

  it("ESC without a revert getter only blurs", () => {
    const action = fieldKeys(input);
    input.value = "kept";
    input.focus();
    press(input, "Escape");
    expect(input.value).toBe("kept");
    expect(document.activeElement).not.toBe(input);
    action.destroy();
  });

  it("Enter blurs to commit and prevents default, but not while composing", () => {
    const action = fieldKeys(input);
    input.focus();
    const composing = press(input, "Enter", { isComposing: true });
    expect(composing.defaultPrevented).toBe(false);
    expect(document.activeElement).toBe(input);
    const enter = press(input, "Enter");
    expect(enter.defaultPrevented).toBe(true);
    expect(document.activeElement).not.toBe(input);
    action.destroy();
  });

  it("does not blur a textarea on Enter", () => {
    const ta = document.createElement("textarea");
    document.body.appendChild(ta);
    const action = fieldKeys(ta);
    ta.focus();
    const e = press(ta, "Enter");
    expect(e.defaultPrevented).toBe(false);
    expect(document.activeElement).toBe(ta);
    action.destroy();
    ta.remove();
  });
});

describe("onGlobalEscape", () => {
  it("fires on ESC when no text field is focused", () => {
    const close = vi.fn();
    const off = onGlobalEscape(close);
    press(window as unknown as Element, "Escape");
    expect(close).toHaveBeenCalledTimes(1);
    off();
    press(window as unknown as Element, "Escape");
    expect(close).toHaveBeenCalledTimes(1);
  });

  it("ignores ESC while a text input holds focus", () => {
    const input = document.createElement("input");
    input.type = "text";
    document.body.appendChild(input);
    input.focus();
    const close = vi.fn();
    const off = onGlobalEscape(close);
    press(window as unknown as Element, "Escape");
    expect(close).not.toHaveBeenCalled();
    off();
    input.remove();
  });
});
