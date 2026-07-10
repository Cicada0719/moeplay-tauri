import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import LibraryCleanupPreview from "./LibraryCleanupPreview.svelte";
import { createMockAiClient } from "../../features/ai/mock";

const preview = {
  id: "change-1",
  taskId: "task-1",
  summary: "建议补充标签",
  confidence: 0.92,
  state: "awaiting_confirmation" as const,
  operations: [{
    selected: true,
    operation: { type: "add_tag" as const, gameId: "game-1", tag: "治愈", reason: "多个本地来源一致" },
  }],
};

describe("LibraryCleanupPreview", () => {
  it("requires per-operation selection and explicit confirmation before apply", async () => {
    const client = createMockAiClient();
    render(LibraryCleanupPreview, { client, initialPreview: preview });

    const operation = await screen.findByRole("checkbox", { name: "选择操作 1" });
    const confirmation = screen.getByRole("checkbox", { name: "确认应用所选操作" });
    const apply = screen.getByRole("button", { name: "确认并应用 0 条" });

    expect(operation).not.toBeChecked();
    expect(confirmation).toBeDisabled();
    expect(apply).toBeDisabled();
    expect(client.calls.applyChangeSet).toHaveLength(0);

    await fireEvent.click(operation);
    expect(screen.getByRole("button", { name: "确认并应用 1 条" })).toBeDisabled();
    expect(client.calls.applyChangeSet).toHaveLength(0);

    await fireEvent.click(confirmation);
    const confirmedApply = screen.getByRole("button", { name: "确认并应用 1 条" });
    expect(confirmedApply).toBeEnabled();
    await fireEvent.click(confirmedApply);

    await waitFor(() => expect(client.calls.applyChangeSet).toHaveLength(1));
    expect(client.calls.applyChangeSet[0]).toEqual({
      changeSetId: "change-1",
      selectedOperationIndexes: [0],
      confirmed: true,
    });
  });

  it("does not expose apply controls for an invalid result", async () => {
    const client = createMockAiClient();
    render(LibraryCleanupPreview, {
      client,
      initialPreview: { ...preview, operations: [{ selected: false, operation: { type: "delete_file", reason: "unsafe" } }] },
    });

    expect(await screen.findByRole("alert")).toHaveTextContent("变更操作无效");
    expect(screen.queryByRole("button", { name: /确认并应用/ })).not.toBeInTheDocument();
    expect(client.calls.applyChangeSet).toHaveLength(0);
  });
});
