import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { clearMockInvokeHandler, setMockInvokeHandler } from "../../../api/core";
import ProviderV2Page from "./ProviderV2Page.svelte";

const provider = {
  id: "komga:https://example.com",
  kind: "komga" as const,
  name: "Komga",
  baseUrl: "https://example.com",
  origin: "https://example.com",
  authMode: "none" as const,
  secretConfigured: false,
  manifest: {
    id: "komga:https://example.com",
    name: "Komga",
    resourceKinds: ["comic"],
    capabilities: ["search", "detail", "chapters", "resolve"],
    trust: "self_hosted",
    version: "batch2",
    enabled: true,
    requiresAuth: false,
    allowedHosts: ["example.com"],
  },
};

const series = { id: "series-1", providerId: provider.id, title: "Provider Test Comic", summary: "summary" };
const chapter = {
  identity: { providerId: provider.id, seriesId: series.id, chapterId: "chapter-1", stableKey: "stable-1" },
  title: "Chapter One",
  sort: { chapterNumber: 1, title: "Chapter One" },
  languageSource: "provider" as const,
  pageCount: 2,
};

afterEach(() => {
  cleanup();
  clearMockInvokeHandler();
});

describe("Comic Provider v2 end-user flow", () => {
  it("searches, loads detail and chapters, resolves, and opens the internal reader", async () => {
    setMockInvokeHandler((command) => {
      if (command === "comic_provider_list") return [provider];
      if (command === "comic_provider_search") return [series];
      if (command === "comic_provider_detail") return { series, alternateTitles: [], genres: ["Test"], totalChapters: 1 };
      if (command === "comic_provider_chapters") return [chapter];
      if (command === "comic_provider_resolve") return {
        mode: "image_pages",
        pages: ["https://example.com/pages/1.jpg", "https://example.com/pages/2.jpg"],
        headers: [],
      };
      throw new Error(`unexpected command: ${command}`);
    });

    render(ProviderV2Page, { props: { onlegacy: vi.fn() } });
    expect(await screen.findByText("当前漫画源")).toBeInTheDocument();

    await fireEvent.input(screen.getByLabelText("搜索 Provider v2 漫画"), { target: { value: "test" } });
    await fireEvent.click(screen.getByRole("button", { name: "搜索" }));
    await fireEvent.click(await screen.findByRole("button", { name: /Provider Test Comic/ }));
    await fireEvent.click(await screen.findByRole("button", { name: /Chapter One/ }));

    expect(await screen.findByRole("dialog", { name: "阅读 Provider Test Comic" })).toBeInTheDocument();
    expect(screen.getByText("1 / 2")).toBeInTheDocument();
  });

  it("shows configuration when no providers exist and preserves the legacy fallback", async () => {
    setMockInvokeHandler((command) => {
      if (command === "comic_provider_list") return [];
      throw new Error(`unexpected command: ${command}`);
    });
    const onlegacy = vi.fn();
    render(ProviderV2Page, { props: { onlegacy } });

    expect(await screen.findByRole("form", { name: "配置 Comic Provider v2" })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("button", { name: /返回普通 \/ PicACG/ }));
    await waitFor(() => expect(onlegacy).toHaveBeenCalledOnce());
  });
});
