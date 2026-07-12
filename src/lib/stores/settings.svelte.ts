import {
  addWatchDir,
  getSettings,
  pickDirectory,
  removeWatchDir,
  updateSettings,
  type Settings,
} from "../api";
import { STARTUP_MIGRATED_KEY, shouldMigrateStartupMode } from "./startup-migration";
import {
  applyAppearance,
  loadStoredAppearance,
  loadStoredTheme,
  type AppTheme,
} from "../utils/theme";
import { normalizeAppearance, type AppearanceSettings } from "../theme-packs";

const defaultSettings: Settings = {
  theme: loadStoredTheme(),
  appearance: loadStoredAppearance(),
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
  scraper_proxy: "",
  ai_enabled: false,
  ai_api_url: "https://api.openai.com/v1/chat/completions",
  ai_model: "gpt-4o-mini",
  nsfw_display_mode: "blur",
  autostart_enabled: false,
  startup_mode: "fullscreen",
  steam_id: undefined,
  home_mascot_enabled: true,
  home_mascot_path: "",
};

type LegacySettingsPayload = Settings & {
  ai_api_key?: unknown;
  steam_api_key?: unknown;
};

function sanitizeSettings(settings: LegacySettingsPayload): Settings {
  const { ai_api_key: _aiApiKey, steam_api_key: _steamApiKey, ...publicSettings } = settings;
  return { ...defaultSettings, ...publicSettings };
}

let _settings = $state<Settings>({ ...defaultSettings });
let _loading = $state(false);
let _loaded = $state(false);

export const settingsStore = {
  get settings() { return _settings; },
  get loading() { return _loading; },
  get loaded() { return _loaded; },
  get theme() { return _settings.theme as AppTheme; },
  get appearance() { return normalizeAppearance(_settings.appearance); },
  get language() { return _settings.language; },

  async load() {
    _loading = true;
    try {
      _settings = sanitizeSettings(await getSettings());
      applyAppearance(_settings.appearance ?? loadStoredAppearance(_settings.theme));
      // 一次性迁移：仅历史默认 dashboard 的老用户迁到 fullscreen 一次；
      // 之后无条件尊重用户选择（避免"普通模式存不住"）。
      const migrated = !!localStorage.getItem(STARTUP_MIGRATED_KEY);
      if (shouldMigrateStartupMode(_settings.startup_mode, migrated)) {
        _settings.startup_mode = "fullscreen";
        updateSettings(_settings).catch(() => {});
      }
      if (!migrated) localStorage.setItem(STARTUP_MIGRATED_KEY, "1");
      _loaded = true;
    } catch (e) {
      console.error("Failed to load settings:", e);
      _settings = { ...defaultSettings };
      applyAppearance(_settings.appearance ?? loadStoredAppearance(_settings.theme));
    } finally {
      _loading = false;
    }
  },

  async save(settings: Settings) {
    _settings = sanitizeSettings(await updateSettings(sanitizeSettings(settings)));
    applyAppearance(_settings.appearance ?? loadStoredAppearance(_settings.theme));
    return _settings;
  },

  async setAppearance(appearance: AppearanceSettings) {
    return this.save({ ..._settings, appearance: normalizeAppearance(appearance) });
  },

  toggleTheme() {
    const color_mode = this.appearance.color_mode === "dark" ? "light" : "dark";
    void this.setAppearance({ ...this.appearance, color_mode });
  },

  setLanguage(lang: string) {
    _settings.language = lang;
  },

  async addWatchDir() {
    try {
      const dir = await pickDirectory();
      if (!dir) return;
      _settings = await addWatchDir(dir);
    } catch (e) {
      console.error("Failed to add watch dir:", e);
    }
  },

  async removeWatchDir(dir: string) {
    try {
      _settings = await removeWatchDir(dir);
    } catch (e) {
      console.error("Failed to remove watch dir:", e);
    }
  },
};
