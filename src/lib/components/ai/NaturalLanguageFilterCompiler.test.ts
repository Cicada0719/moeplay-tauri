import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import NaturalLanguageFilterCompiler from "./NaturalLanguageFilterCompiler.svelte";
import { createMockAiClient } from "../../features/ai/mock";

async function typeInto(element: HTMLElement, value: string) {
  await fireEvent.input(element, { target: { value } });
}

describe("NaturalLanguageFilterCompiler", () => {
  it("never exposes apply for an invalid AI result", async () => {
    const onApply = vi.fn();
    const client = createMockAiClient({
      compileResult: (request) => ({
        taskId: "unsafe-task",
        generation: request.generation,
        schemaId: "natural_language_filter.dsl",
        schemaVersion: "1.0.0",
        dsl: {
          kind: "game",
          filters: [{ field: "sql", op: "execute", value: "DROP TABLE games" }],
          sort: [],
          explanation: "unsafe",
        },
      }),
    });
    render(NaturalLanguageFilterCompiler, { client, onApply });

    await typeInto(screen.getByLabelText("描述你想找的作品"), "删除所有资料");
    await fireEvent.click(screen.getByRole("button", { name: "编译为本地 DSL" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("禁止的执行语义");
    expect(screen.queryByRole("button", { name: "应用筛选" })).not.toBeInTheDocument();
    expect(onApply).not.toHaveBeenCalled();
  });

  it("builds and applies a validated structured fallback without AI", async () => {
    const onApply = vi.fn();
    const client = createMockAiClient({ error: new Error("offline") });
    render(NaturalLanguageFilterCompiler, { client, onApply });

    await fireEvent.click(screen.getByRole("button", { name: "本地筛选" }));
    await typeInto(screen.getByLabelText("标题关键词"), "夏日");
    await typeInto(screen.getByLabelText("标签"), "治愈");
    await fireEvent.click(screen.getByRole("checkbox", { name: "仅未玩过" }));
    await fireEvent.click(screen.getByRole("button", { name: "生成本地 DSL" }));
    await fireEvent.click(await screen.findByRole("button", { name: "应用筛选" }));

    await waitFor(() => expect(onApply).toHaveBeenCalledTimes(1));
    expect(onApply.mock.calls[0][0].filters).toEqual(expect.arrayContaining([
      { field: "title", op: "contains", value: "夏日" },
      { field: "tags", op: "contains_any", value: ["治愈"] },
      { field: "lastPlayedAt", op: "is_null" },
    ]));
    expect(client.calls.compileFilter).toHaveLength(0);
  });
});
