import type { Game } from "./gameLibrary.svelte";

let _selectedId = $state<string | null>(null);
let _selectedIds = $state<Set<string>>(new Set());

export const selectionStore = {
  get selectedId() { return _selectedId; },
  get selectedGame() {
    return _selectedId;
  },
  get selectedIds() { return _selectedIds; },
  get selectionMode() { return _selectedIds.size > 0; },

  selectGame(id: string | null) {
    _selectedId = id;
  },

  toggleSelection(id: string) {
    const next = new Set(_selectedIds);
    if (next.has(id)) next.delete(id); else next.add(id);
    _selectedIds = next;
  },

  selectAll(ids: string[]) {
    _selectedIds = new Set(ids);
  },

  clearSelection() {
    _selectedIds = new Set();
  },

  isSelected(id: string) {
    return _selectedIds.has(id);
  },
};

/** Resolve selected game object from a game list. */
export function resolveSelectedGame(games: Game[], id: string | null): Game | null {
  if (!id) return null;
  return games.find(g => g.id === id) ?? null;
}
