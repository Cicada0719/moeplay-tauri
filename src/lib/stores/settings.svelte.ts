import {
  addWatchDir,
  getSettings,
  pickDirectory,
  removeWatchDir,
  updateSettings,
  type Settings,
} from "../api";

const defaultSettings: Settings = {
  theme: localStorage.getItem("moegame-theme") || "dark",
  watch_dirs: [],
  auto_scrape: true,
  language: "zh",
  minimize_to_tray: false,
  vndb_enabled: true,
  bangumi_enabled: true,
  ai_enabled: false,
  ai_api_url: "https://api.openai.com/v1/chat/completions",
  ai_api_key: "",
  ai_model: "gpt-4o-mini",
  nsfw_display_mode: "blur",
  autostart_enabled: false,
  startup_mode: "dashboard",
  steam_id: undefined,
  steam_api_key: undefined,
};

let _settings = $state<Settings>({ ...defaultSettings });
let _loading = $state(false);

function applyTheme(theme: string) {
  localStorage.setItem("moegame-theme", theme);
  document.documentElement.setAttribute("data-theme", theme);
}

export const settingsStore = {
  get settings() { return _settings; },
  get loading() { return _loading; },
  get theme() { return _settings.theme; },
  get language() { return _settings.language; },

  async load() {
    _loading = true;
    try {
      _settings = { ...defaultSettings, ...(await getSettings()) };
      applyTheme(_settings.theme);
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
