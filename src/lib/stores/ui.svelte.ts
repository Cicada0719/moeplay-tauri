export type ViewChangeSource = "direct" | "router";

export interface ViewChange {
  previous: string;
  current: string;
  source: ViewChangeSource;
}

type ViewChangeListener = (change: ViewChange) => void;

let _currentView = $state("home");
let _viewMode = $state<"grid" | "list" | "compact">("grid");
let _sortBy = $state("recent");
let _sidebarCollapsed = $state(false);
let _showFirstRunWizard = $state(false);
let _showDetailPanel = $state(false);
let _showScrapeDialog = $state(false);
let _scrapeTargetGameId = $state<string | null>(null);
let _toasts = $state<{ id: number; message: string; type: "info" | "success" | "error" }[]>([]);
let _toastId = 0;
let _pendingDownloadUrl = $state<string | null>(null);
let _pendingDownloadName = $state<string | null>(null);
let _bigPictureActive = $state(false);
let _libraryMode = $state<"home" | "all">("home");
let _drawerOpen = $state(false);
let _drawerView = $state<string | null>(null);
let _focusSearchSignal = $state(0);
let _focusSearchScope = $state<string | null>(null);
const viewChangeListeners = new Set<ViewChangeListener>();

function setCurrentView(view: string, source: ViewChangeSource) {
  const next = view.trim() || "home";
  if (next === _currentView) return;
  const previous = _currentView;
  _currentView = next;
  const change = { previous, current: next, source } satisfies ViewChange;
  for (const listener of [...viewChangeListeners]) listener(change);
}

export const uiStore = {
  get currentView() { return _currentView; },
  set currentView(v: string) { setCurrentView(v, "direct"); },
  setCurrentViewFromRouter(v: string) { setCurrentView(v, "router"); },
  subscribeViewChanges(listener: ViewChangeListener) {
    viewChangeListeners.add(listener);
    return () => viewChangeListeners.delete(listener);
  },
  get viewMode() { return _viewMode; },
  set viewMode(v: "grid" | "list" | "compact") { _viewMode = v; },
  get sortBy() { return _sortBy; },
  set sortBy(v: string) { _sortBy = v; },
  get sidebarCollapsed() { return _sidebarCollapsed; },
  set sidebarCollapsed(v: boolean) { _sidebarCollapsed = v; },
  get showFirstRunWizard() { return _showFirstRunWizard; },
  set showFirstRunWizard(v: boolean) { _showFirstRunWizard = v; },
  get showDetailPanel() { return _showDetailPanel; },
  set showDetailPanel(v: boolean) { _showDetailPanel = v; },
  get showScrapeDialog() { return _showScrapeDialog; },
  get scrapeTargetGameId() { return _scrapeTargetGameId; },
  get toasts() { return _toasts; },
  get notifications() { return _toasts; },
  get bigPictureActive() { return _bigPictureActive; },
  get libraryMode() { return _libraryMode; },
  set libraryMode(v: "home" | "all") { _libraryMode = v; },
  toggleBigPicture() {
    this.setBigPicture(!_bigPictureActive);
  },
  setBigPicture(v: boolean) {
    _bigPictureActive = Boolean(v);
  },

  toast(message: string, type: "info" | "success" | "error" = "info") {
    const id = ++_toastId;
    _toasts = [..._toasts, { id, message, type }];
    setTimeout(() => {
      _toasts = _toasts.filter(t => t.id !== id);
    }, 3000);
  },

  notify(message: string, type: "info" | "success" | "error" = "info") {
    this.toast(message, type);
  },

  openScrapeDialog(gameId: string) {
    _scrapeTargetGameId = gameId;
    _showScrapeDialog = true;
  },

  closeScrapeDialog() {
    _showScrapeDialog = false;
    _scrapeTargetGameId = null;
  },

  get drawerOpen() { return _drawerOpen; },
  get drawerView() { return _drawerView; },
  openDrawer(view: string) {
    _drawerView = view;
    _drawerOpen = true;
  },
  closeDrawer() {
    _drawerOpen = false;
    _drawerView = null;
  },

  get pendingDownloadUrl() { return _pendingDownloadUrl; },
  set pendingDownloadUrl(v: string | null) { _pendingDownloadUrl = v; },
  get pendingDownloadName() { return _pendingDownloadName; },
  set pendingDownloadName(v: string | null) { _pendingDownloadName = v; },

  get focusSearchSignal() { return _focusSearchSignal; },
  get focusSearchScope() { return _focusSearchScope; },
  requestFocusSearch(scope = _currentView) {
    _focusSearchScope = scope;
    _focusSearchSignal++;
  },
  consumeFocusSearchSignal() {
    _focusSearchSignal = 0;
    _focusSearchScope = null;
  },
};
