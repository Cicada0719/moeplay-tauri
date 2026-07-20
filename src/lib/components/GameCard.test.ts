import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import type { Game } from "../stores/games.svelte";

const mocks = vi.hoisted(() => ({
  navigateTo: vi.fn(),
  openOverlay: vi.fn(),
  closeOverlay: vi.fn(),
  selectGame: vi.fn(),
  toggleSelection: vi.fn(),
  toggleFavorite: vi.fn(async () => undefined),
  deleteGame: vi.fn(async () => undefined),
  notify: vi.fn(),
}));

vi.mock("../stores/router.svelte", () => ({
  navigateTo: mocks.navigateTo,
  openOverlay: mocks.openOverlay,
  closeOverlay: mocks.closeOverlay,
}));

vi.mock("../stores/games.svelte", () => ({
  gameStore: {
    selectionMode: false,
    isSelected: () => false,
    selectGame: mocks.selectGame,
    toggleSelection: mocks.toggleSelection,
    toggleFavorite: mocks.toggleFavorite,
    deleteGame: mocks.deleteGame,
  },
}));

vi.mock("../stores/ui.svelte", () => ({
  uiStore: {
    viewMode: "grid",
    libraryMode: "all",
    notify: mocks.notify,
  },
}));

vi.mock("../stores/settings.svelte", () => ({
  settingsStore: { settings: { nsfw_display_mode: "show" } },
}));

import GameCard from "./GameCard.svelte";

const game = {
  id: "game-1",
  name: "星海回声",
  exe_path: "C:\\Games\\Echoes\\echoes.exe",
  created_at: "2026-07-01T08:00:00.000Z",
  updated_at: "2026-07-10T10:00:00.000Z",
  screenshots: [],
  favorite: false,
  hidden: false,
  tags: ["视觉小说"],
  metadata: { developer: "Fixture Studio", release_year: 2026 },
  play_tracker: { completion_status: "playing", user_rating: 8 },
  save_data: {},
  aliases: [],
  tag_entries: [],
  play_time_seconds: 0,
} as unknown as Game;

describe("GameCard UI-v2 contract", () => {
  it("keeps selected, disabled and loading as independent states", () => {
    const selected = render(GameCard, { props: { game, selected: true, disabled: false, loading: false } });
    const selectedArticle = selected.container.querySelector<HTMLElement>("[data-ui-v2='media-card']")!;
    const selectedPrimary = screen.getByRole("button", { name: "打开 星海回声 详情" });
    expect(selectedArticle).toHaveAttribute("data-selected", "true");
    expect(selectedArticle).toHaveAttribute("aria-busy", "false");
    expect(selectedPrimary).toBeEnabled();
    selected.unmount();

    const disabled = render(GameCard, { props: { game, selected: false, disabled: true, loading: false } });
    const disabledArticle = disabled.container.querySelector<HTMLElement>("[data-ui-v2='media-card']")!;
    expect(disabledArticle).not.toHaveAttribute("data-selected");
    expect(screen.getByRole("button", { name: "打开 星海回声 详情" })).toBeDisabled();
    disabled.unmount();

    const loading = render(GameCard, { props: { game, selected: false, disabled: false, loading: true } });
    const loadingArticle = loading.container.querySelector<HTMLElement>("[data-ui-v2='media-card']")!;
    expect(loadingArticle).toHaveAttribute("aria-busy", "true");
    expect(loadingArticle).not.toHaveAttribute("data-selected");
  });

  it("stores a stable focus key and opens detail through the router", async () => {
    render(GameCard, { props: { game } });
    const primary = screen.getByRole("button", { name: "打开 星海回声 详情" });
    await waitFor(() => expect(primary).toHaveAttribute("data-focus-key", "game-card-game-1"));
    await fireEvent.click(primary);

    expect(mocks.selectGame).toHaveBeenCalledWith("game-1");
    expect(mocks.navigateTo).toHaveBeenCalledWith("game-detail", {
      entity: { kind: "game", id: "game-1" },
      focus: "start",
    });
  });

  it("uses a cancel-first alert dialog and restores focus on Escape", async () => {
    render(GameCard, { props: { game } });
    const trigger = screen.getByRole("button", { name: "删除 星海回声" });
    trigger.focus();
    await fireEvent.click(trigger);

    const dialog = await screen.findByRole("alertdialog", { name: "从游戏库删除？" });
    expect(dialog).toBeInTheDocument();
    const dialogRoot = screen.getByTestId("delete-dialog-game-1");
    expect(dialogRoot.parentElement).toBe(document.body);
    const cancel = screen.getByRole("button", { name: "取消" });
    await waitFor(() => expect(cancel).toHaveFocus());
    expect(mocks.openOverlay).toHaveBeenCalled();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
    await waitFor(() => expect(trigger).toHaveFocus());
    expect(mocks.deleteGame).not.toHaveBeenCalled();
  });
});
