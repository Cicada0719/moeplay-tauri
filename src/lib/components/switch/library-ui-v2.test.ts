import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import type { Game } from "../../stores/games.svelte";

vi.mock("./useGamepad.svelte", () => ({ attachGamepad: () => Object.assign(() => {}, {}) }));
vi.mock("gsap", () => ({ gsap: { context: () => ({ revert: () => {} }), from: () => {} } }));

import TileCard from "./TileCard.svelte";
import TileRail from "./TileRail.svelte";

const games = [
  { id: "game-a", name: "星海回声", favorite: false, tags: [], metadata: {}, play_tracker: {}, save_data: {}, screenshots: [] },
  { id: "game-b", name: "夏日列车", favorite: true, tags: [], metadata: {}, play_tracker: {}, save_data: {}, screenshots: [] },
] as unknown as Game[];

describe("library UI-v2 migration contract", () => {
  it("keeps every scoped production component on the frozen UI-v2 surface", () => {
    const files = {
      SwitchHome: readFileSync(resolve("src/lib/components/switch/SwitchHome.svelte"), "utf8"),
      GameGrid: readFileSync(resolve("src/lib/components/GameGrid.svelte"), "utf8"),
      GameCard: readFileSync(resolve("src/lib/components/GameCard.svelte"), "utf8"),
      GameDetail: readFileSync(resolve("src/lib/components/GameDetailPage.svelte"), "utf8"),
      TileCard: readFileSync(resolve("src/lib/components/switch/TileCard.svelte"), "utf8"),
    };

    expect(files.SwitchHome).toContain('<PageShell as="div"');
    expect(files.SwitchHome).toContain("<PageHeader");
    expect(files.SwitchHome).toContain("<FilterBar");
    expect(files.SwitchHome).toContain("<AsyncState");
    expect(files.GameGrid).toContain("<ContentGrid");
    expect(files.GameGrid).toContain("data-route-scroll");
    expect(files.GameCard).toContain("<MediaCard");
    expect(files.TileCard).toContain("<MediaCard");
    expect(files.GameDetail).toContain("<DetailPanel");
    expect(files.GameDetail).toContain('class="hero-contact-sheet"');
    expect(files.GameDetail).toContain("width: 100vw");
    expect(files.GameCard).not.toContain("window.confirm");
  });

  it("gives tile cards stable roving focus and separate states", async () => {
    const pick = vi.fn();
    const launch = vi.fn();
    const view = render(TileCard, {
      props: {
        game: games[0],
        selected: true,
        disabled: false,
        loading: false,
        focusKey: "game-card-game-a",
        tabIndex: 0,
        onpick: pick,
        onlaunch: launch,
      },
    });

    const button = screen.getByRole("button", { name: "打开 星海回声" });
    await waitFor(() => {
      expect(button).toHaveAttribute("data-focus-key", "game-card-game-a");
      expect(button).toHaveAttribute("tabindex", "0");
    });
    expect(view.container.querySelector("[data-selected='true']")).toBeInTheDocument();

    await fireEvent.keyDown(button, { key: "Enter" });
    expect(pick).toHaveBeenCalledOnce();
    await fireEvent.keyDown(button, { key: " " });
    expect(launch).toHaveBeenCalledOnce();
  });

  it("moves real DOM focus through the recent rail", async () => {
    const onselect = vi.fn();
    const onactivate = vi.fn();
    render(TileRail, {
      props: {
        items: games,
        selectedId: "game-a",
        onselect,
        onactivate,
        onlaunch: vi.fn(),
        onshowall: vi.fn(),
      },
    });

    const first = screen.getByRole("button", { name: "打开 星海回声" });
    const second = screen.getByRole("button", { name: "打开 夏日列车" });
    first.focus();
    await fireEvent.keyDown(first, { key: "ArrowRight" });
    await waitFor(() => expect(second).toHaveFocus());
    expect(onselect).toHaveBeenLastCalledWith("game-b");

    await fireEvent.keyDown(second, { key: "Enter" });
    expect(onactivate).toHaveBeenCalledWith("game-b");
  });
});


