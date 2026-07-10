import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import LibraryFeatureToggle from "./LibraryFeatureToggle.svelte";
import { LIBRARY_V2_FLAG, parseLibraryV2Flag, readLibraryV2Flag } from "./feature-flag";

afterEach(() => {
  cleanup();
  localStorage.clear();
});

describe("Library v2 feature flag", () => {
  it("switches from the legacy fallback to Library v2 and persists the flag", async () => {
    const onChange = vi.fn();
    render(LibraryFeatureToggle, { props: { enabled: false, onChange } });

    expect(screen.getByText("旧导入流程 fallback")).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("switch", { name: "启用 Library v2" }));

    expect(localStorage.getItem(LIBRARY_V2_FLAG)).toBe("1");
    expect(onChange).toHaveBeenCalledWith(true);
    expect(readLibraryV2Flag()).toBe(true);
  });

  it("accepts explicit enabled values while keeping an absent flag on the old flow", () => {
    expect(parseLibraryV2Flag(null)).toBe(false);
    expect(parseLibraryV2Flag("0")).toBe(false);
    expect(parseLibraryV2Flag("enabled")).toBe(true);
    expect(parseLibraryV2Flag("true")).toBe(true);
  });
});
