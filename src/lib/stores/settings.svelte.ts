import {
  addWatchDir,
  getSettings,
  pickDirectory,
  removeWatchDir,
  updateSettings,
  type Settings,
} from "../api";
import { STARTUP_MIGRATED_KEY, shouldMigrateStartupMode } from "./startup-migration";

const defaultSettings: Settings = {
  theme: localStorage.getItem("moegame-theme") || "dark",
  watch_dirs: [],
  auto_scrape: true,
  language: "zh",
  minimize_to_tray: false,
  vndb_enabled: true,
  bangumi_enabled: true,
  dlsite_enabled: true,
  touchgal_enabled: true,
  erogamescape_enabled: true,
  ymgal_enabled: true,
  kungal_enabled: true,
  steam_enabled: true,
  pcgw_enabled: true,
  ai_enabled: false,
  ai_api_url: "https://api.openai.com/v1/chat/completions",
  ai_api_key: "",
  ai_model: "gpt-4o-mini",
  nsfw_display_mode: "blur",
  autostart_enabled: false,
  startup_mode: "fullscreen",
  steam_id: undefined,
  steam_api_key: undefined,
};

let _settings = $state<Settings>({ ...defaultSettings });
let _loading = $state(false);
let _loaded = $state(false);

function applyTheme(theme: string) {
  localStorage.setItem("moegame-theme", theme);
  document.documentElement.setAttribute("data-theme", theme);
}

export const settingsStore = {
  get settings() { return _settings; },
  get loading() { return _loading; },
  get loaded() { return _loaded; },
  get theme() { return _settings.theme; },
  get language() { return _settings.language; },

  async load() {
    _loading = true;
    try {
      _settings = { ...defaultSettings, ...(await getSettings()) };
      applyTheme(_settings.theme);
      // 一次性迁移：仅历史默认 dashboard 的老用户迁到 fullscreen 一次；
      // 之后无条件尊重用户选择（避免"普通模式存不住"）。
      const migrated = !!localStorage.getItem(STARTUP_MIGRATED_KEY);
      if (shouldMigrateStartupMode(_settings.startup_mode, migrated)) {
        _settings.startup_mode = "fullscreen";
        updateSettings(_settings).catch(() => {});
      }
      if (!migrated) localStorage.setItem(STARTUP_MIGRATED_KEY, "1");
      _loaded = true;
    } finally {
      _loading = false;
    }
  },

  async save(settings: Settings) {
    _settings = await updateSettings({ ...defaultSettings, ...settings });
    applyTheme(_settings.theme);
    return _settings;
  },

  toggleTheme() {
    const theme = _settings.theme === "dark" ? "light" : "dark";
    void this.save({ ..._settings, theme });
  },

  setLanguage(lang: string) {
    _settings.language = lang;
  },

  async addWatchDir() {
    const dir = await pickDirectory();
    _settings = await addWatchDir(dir);
  },

  async removeWatchDir(dir: string) {
    _settings = await removeWatchDir(dir);
  },
};
