import { libraryStore, type Game, type SmartCollection } from "./gameLibrary.svelte";
import { selectionStore } from "./gameSelection.svelte";

export type { Game, SmartCollection };
export { matchesPinyin } from "./gameLibrary.svelte";

export const gameStore = {
  // ---- reactive (library) ----
  get games() { return libraryStore.games; },
  get allGames() { return libraryStore.allGames; },
  get installedGames() { return libraryStore.installedGames; },
  get availabilityGames() { return libraryStore.availabilityGames; },
  get availabilityScope() { return libraryStore.availabilityScope; },
  set availabilityScope(scope: import("./gameLibrary.svelte").GameAvailabilityScope) { libraryStore.availabilityScope = scope; },
  get loading() { return libraryStore.loading; },
  get loadError() { return libraryStore.loadError; },
  get searchQuery() { return libraryStore.searchQuery; },
  set searchQuery(v: string) { libraryStore.searchQuery = v; },
  get filterTag() { return libraryStore.filterTag; },
  set filterTag(t: string | null) { libraryStore.filterTag = t; },
  get quickFilter() { return libraryStore.quickFilter; },
  set quickFilter(f: string | null) { libraryStore.quickFilter = f; },
  get sortBy() { return libraryStore.sortBy; },
  set sortBy(v: string) { libraryStore.sortBy = v; },

  // ---- reactive (selection) ----
  get selectedId() { return selectionStore.selectedId; },
  get selectedGame() {
    return libraryStore.allGames.find(g => g.id === selectionStore.selectedId) ?? null;
  },
  get selectedIds() { return selectionStore.selectedIds; },
  get selectionMode() { return selectionStore.selectionMode; },

  // ---- smart collections ----
  get smartCollections() { return libraryStore.smartCollections; },
  get activeCollectionId() { return libraryStore.activeCollectionId; },
  activateCollection(id: string | null) { libraryStore.activateCollection(id); },
  addCollection(name: string, icon: string, filters: SmartCollection["filters"]) {
    return libraryStore.addCollection(name, icon, filters);
  },
  updateCollection(id: string, updates: Partial<SmartCollection>) {
    libraryStore.updateCollection(id, updates);
  },
  removeCollection(id: string) { libraryStore.removeCollection(id); },
  saveCurrentAsCollection(name: string, icon: string) {
    return libraryStore.saveCurrentAsCollection(name, icon);
  },
  exportCollection(id: string) { return libraryStore.exportCollection(id); },
  importCollection(json: string) { return libraryStore.importCollection(json); },
  duplicateCollection(id: string) { return libraryStore.duplicateCollection(id); },

  // ---- data loading ----
  load() { return libraryStore.load(); },
  search(query: string) { return libraryStore.search(query); },
  importGame() { return libraryStore.importGame(); },

  // ---- selection ----
  selectGame(id: string | null) {
    selectionStore.selectGame(id);
  },
  toggleSelection(id: string) { selectionStore.toggleSelection(id); },
  selectAll() { selectionStore.selectAll(libraryStore.games.map(g => g.id)); },
  clearSelection() { selectionStore.clearSelection(); },
  isSelected(id: string) { return selectionStore.isSelected(id); },

  // ---- game actions ----
  launch(id: string) { return libraryStore.launch(id); },
  launchWithResult(id: string) { return libraryStore.launchWithResult(id); },
  launchJP(id: string) { return libraryStore.launchJP(id); },
  async toggleFavorite(id: string) { return libraryStore.toggleFavorite(id); },
  toggleFav(id: string) { return libraryStore.toggleFavorite(id); },
  async deleteGame(id: string) {
    await libraryStore.deleteGame(id);
    selectionStore.clearSelection();
  },
  remove(id: string) { return this.deleteGame(id); },
  updateSaveDir(id: string, path: string | null) { return libraryStore.updateSaveDir(id, path); },
  scrape(id: string, result: import("../api").ScrapeResult) { return libraryStore.scrape(id, result); },

  // ---- batch actions ----
  async batchDelete() {
    const ids = [...selectionStore.selectedIds];
    selectionStore.clearSelection();
    return libraryStore.batchDelete(ids);
  },
  async batchToggleHidden() {
    const ids = [...selectionStore.selectedIds];
    selectionStore.clearSelection();
    return libraryStore.batchToggleHidden(ids);
  },
  async batchToggleFavorite() {
    const ids = [...selectionStore.selectedIds];
    selectionStore.clearSelection();
    return libraryStore.batchToggleFavorite(ids);
  },
  async batchAddTag(tag: string) {
    const ids = [...selectionStore.selectedIds];
    selectionStore.clearSelection();
    return libraryStore.batchAddTag(ids, tag);
  },
  async batchSetStatus(status: import("../api").CompletionStatus) {
    const ids = [...selectionStore.selectedIds];
    selectionStore.clearSelection();
    return libraryStore.batchSetStatus(ids, status);
  },
};
