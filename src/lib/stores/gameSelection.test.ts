import { describe, it, expect, beforeEach } from "vitest";
import { selectionStore, resolveSelectedGame } from "./gameSelection.svelte";
import type { Game } from "./gameLibrary.svelte";

function makeGame(name: string): Game {
  return { id: crypto.randomUUID(), name } as Game;
}

describe("selectionStore", () => {
  beforeEach(() => {
    selectionStore.clearSelection();
    selectionStore.selectGame(null);
  });

  it("selects a game", () => {
    selectionStore.selectGame("g1");
    expect(selectionStore.selectedId).toBe("g1");
  });

  it("toggles selection", () => {
    selectionStore.toggleSelection("g1");
    expect(selectionStore.isSelected("g1")).toBe(true);
    selectionStore.toggleSelection("g1");
    expect(selectionStore.isSelected("g1")).toBe(false);
  });

  it("enters selection mode when multiple items selected", () => {
    expect(selectionStore.selectionMode).toBe(false);
    selectionStore.toggleSelection("g1");
    selectionStore.toggleSelection("g2");
    expect(selectionStore.selectionMode).toBe(true);
  });

  it("selects all from list", () => {
    selectionStore.selectAll(["g1", "g2", "g3"]);
    expect(selectionStore.selectedIds.size).toBe(3);
  });

  it("clears selection", () => {
    selectionStore.selectAll(["g1", "g2"]);
    selectionStore.clearSelection();
    expect(selectionStore.selectedIds.size).toBe(0);
    expect(selectionStore.selectionMode).toBe(false);
  });
});

describe("resolveSelectedGame", () => {
  it("resolves game by id", () => {
    const games = [makeGame("A"), makeGame("B")];
    expect(resolveSelectedGame(games, games[1].id)).toEqual(games[1]);
  });

  it("returns null for missing id", () => {
    expect(resolveSelectedGame([], "x")).toBeNull();
    expect(resolveSelectedGame([makeGame("A")], null)).toBeNull();
  });
});
