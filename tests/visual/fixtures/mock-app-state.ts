export const FIXED_NOW = "2026-07-10T12:00:00.000Z";
export const FIXED_RANDOM_SEED = 0x12_01_20_26;

export interface MockGame {
  id: string;
  name: string;
  exe_path: string;
  created_at: string;
  updated_at: string;
  screenshots: string[];
  favorite: boolean;
  hidden: boolean;
  tags: string[];
  metadata: Record<string, unknown>;
  play_tracker: Record<string, unknown>;
  save_data: Record<string, unknown>;
  aliases: unknown[];
  tag_entries: unknown[];
  play_time_seconds: number;
  [key: string]: unknown;
}

export interface MockSettings {
  theme: string;
  watch_dirs: string[];
  auto_scrape: boolean;
  language: string;
  minimize_to_tray: boolean;
  vndb_enabled: boolean;
  bangumi_enabled: boolean;
  dlsite_enabled: boolean;
  touchgal_enabled: boolean;
  erogamescape_enabled: boolean;
  ymgal_enabled: boolean;
  kungal_enabled: boolean;
  steam_enabled: boolean;
  pcgw_enabled: boolean;
  scraper_proxy: string;
  ai_enabled: boolean;
  ai_api_url: string;
  ai_model: string;
  nsfw_display_mode: "show" | "blur" | "hide";
  autostart_enabled: boolean;
  startup_mode: string;
  steam_id: string;
  [key: string]: unknown;
}

export interface MockAppState {
  settings: MockSettings;
  games: MockGame[];
  commandResults?: Record<string, unknown>;
  localStorage?: Record<string, string>;
}

export const MOCK_SETTINGS: MockSettings = {
  theme: "dark",
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
  steam_id: "",
};

export const MOCK_GAMES: MockGame[] = [
  {
    id: "fixture-game-1",
    name: "星海回声",
    exe_path: "C:\\Games\\Echoes\\echoes.exe",
    created_at: "2026-07-01T08:00:00.000Z",
    updated_at: "2026-07-10T10:00:00.000Z",
    screenshots: [],
    favorite: true,
    hidden: false,
    tags: ["视觉小说", "科幻"],
    metadata: { developer: "Fixture Studio", platform: "pc", release_year: 2026 },
    play_tracker: {
      total_seconds: 7_200,
      sessions: [],
      completion_status: "playing",
      last_played: "2026-07-10T10:00:00.000Z",
      user_rating: 8,
    },
    save_data: { paths: [], backups: [] },
    aliases: [],
    tag_entries: [],
    play_time_seconds: 7_200,
  },
  {
    id: "fixture-game-2",
    name: "夏日列车",
    exe_path: "C:\\Games\\SummerTrain\\train.exe",
    created_at: "2026-06-20T08:00:00.000Z",
    updated_at: "2026-07-08T10:00:00.000Z",
    screenshots: [],
    favorite: false,
    hidden: false,
    tags: ["剧情", "治愈"],
    metadata: { developer: "Fixture Works", platform: "pc", release_year: 2025 },
    play_tracker: {
      total_seconds: 1_800,
      sessions: [],
      completion_status: "not_started",
      last_played: null,
      user_rating: 0,
    },
    save_data: { paths: [], backups: [] },
    aliases: [],
    tag_entries: [],
    play_time_seconds: 1_800,
  },
];

export const EMPTY_APP_STATE: MockAppState = {
  settings: { ...MOCK_SETTINGS },
  games: [],
  localStorage: {
    "moegame-startup-migrated-v1": "1",
    "moeplay-theme": "dark",
  },
};

export const DEFAULT_APP_STATE: MockAppState = {
  settings: { ...MOCK_SETTINGS },
  games: MOCK_GAMES.map((game) => ({ ...game })),
  localStorage: {
    "moegame-startup-migrated-v1": "1",
    "moeplay-theme": "dark",
  },
};
