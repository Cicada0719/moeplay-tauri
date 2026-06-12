import { expect, test, type Page, type TestInfo } from "@playwright/test";

type MockMode = "seeded" | "empty" | "large";

const fixedNow = "2026-06-11T08:00:00.000Z";

function svgDataUrl(title: string, palette: string[], aspect: "poster" | "backdrop") {
  const [bg, accent, ink] = palette;
  const [width, height] = aspect === "poster" ? [450, 600] : [1280, 720];
  const svg = `
    <svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
      <defs>
        <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stop-color="${bg}"/>
          <stop offset="1" stop-color="${accent}"/>
        </linearGradient>
      </defs>
      <rect width="${width}" height="${height}" fill="url(#g)"/>
      <circle cx="${width * 0.78}" cy="${height * 0.22}" r="${width * 0.18}" fill="${ink}" opacity=".18"/>
      <rect x="${width * 0.08}" y="${height * 0.1}" width="${width * 0.84}" height="${height * 0.8}" rx="24" fill="none" stroke="${ink}" stroke-width="8" opacity=".42"/>
      <text x="${width * 0.1}" y="${height * 0.58}" fill="${ink}" font-family="Arial, sans-serif" font-size="${aspect === "poster" ? 46 : 72}" font-weight="700">${title}</text>
    </svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

const visualArt = {
  asterismCover: svgDataUrl("ASTR", ["#101827", "#e8557f", "#f8fafc"], "poster"),
  asterismBackground: svgDataUrl("Asterism Script", ["#07090f", "#31415f", "#f8fafc"], "backdrop"),
  moonlitCover: svgDataUrl("MOON", ["#111827", "#4f46e5", "#eef2ff"], "poster"),
  moonlitBackground: svgDataUrl("Moonlit Archive", ["#050816", "#1d4ed8", "#e0f2fe"], "backdrop"),
  gardenCover: svgDataUrl("GARDEN", ["#0f172a", "#16a34a", "#ecfdf5"], "poster"),
  gardenBackground: svgDataUrl("Garden Protocol", ["#07120f", "#166534", "#dcfce7"], "backdrop"),
};

const baseMetadata = {
  developer: "Moe Lab",
  publisher: "Moe Lab",
  platform: "pc",
  engine: "KiriKiri",
  genres: ["Visual Novel"],
  languages: ["zh"],
  voice_languages: ["ja"],
  version: "1.0",
  original_name: "Moe Sample",
  homepage: undefined,
  developer_homepage: undefined,
  stores: [],
  age_rating: "全年龄",
  series: undefined,
  release_date: "2026-01-10",
  release_year: 2026,
  estimated_hours: 18,
  vndb_rating: 8.2,
  bangumi_rating: 7.8,
  vndb_id: "v1000",
  bangumi_id: undefined,
  cover: undefined,
  background: undefined,
};

const baseTracker = {
  total_seconds: 0,
  sessions: [],
  completion_status: "not_started",
  last_played: undefined,
  first_played: undefined,
  user_rating: undefined,
  review: undefined,
  achievements_total: 0,
  achievements_unlocked: 0,
  finished: false,
  completion_count: 0,
};

const baseSaveData = {
  save_dir: undefined,
  auto_backup: false,
  backup_interval_minutes: 30,
  max_backups: 5,
  backups: [],
  cloud_sync: false,
  cloud_provider: undefined,
};

const seededGames = [
  {
    id: "visual-1",
    name: "Asterism Script",
    exe_path: "D:\\Games\\Asterism\\Asterism.exe",
    install_dir: "D:\\Games\\Asterism",
    game_type: "visual_novel",
    library_source: "steam",
    library_id: "10001",
    launch_uri: "steam://rungameid/10001",
    last_imported_at: fixedNow,
    platform: "steam",
    created_at: "2026-06-01T12:00:00.000Z",
    updated_at: fixedNow,
    description: "A seeded visual smoke item that keeps the home shell deterministic.",
    cover: visualArt.asterismCover,
    background: visualArt.asterismBackground,
    icon: undefined,
    screenshots: [visualArt.asterismBackground],
    favorite: true,
    hidden: false,
    tags: ["Visual Novel", "Story"],
    genres: ["Visual Novel"],
    metadata: {
      ...baseMetadata,
      original_name: "ASTR",
      vndb_rating: 8.7,
      vndb_id: "v1001",
      cover: visualArt.asterismCover,
      background: visualArt.asterismBackground,
    },
    play_tracker: {
      ...baseTracker,
      total_seconds: 14_400,
      completion_status: "playing",
      last_played: "2026-06-10T20:30:00.000Z",
      user_rating: 8.8,
      achievements_total: 24,
      achievements_unlocked: 8,
    },
    save_data: baseSaveData,
    aliases: [],
    tag_entries: [],
    play_time_seconds: 14_400,
    last_played: "2026-06-10T20:30:00.000Z",
    rating: 8.8,
    release_year: 2026,
  },
  {
    id: "visual-2",
    name: "Moonlit Archive",
    exe_path: "D:\\Games\\Moonlit\\Moonlit.exe",
    install_dir: "D:\\Games\\Moonlit",
    game_type: "visual_novel",
    library_source: "epic",
    library_id: "moonlit",
    launch_uri: "com.epicgames.launcher://apps/moonlit?action=launch",
    last_imported_at: fixedNow,
    platform: "epic",
    created_at: "2026-06-05T12:00:00.000Z",
    updated_at: fixedNow,
    description: "Second deterministic item for rail and platform grouping checks.",
    cover: visualArt.moonlitCover,
    background: visualArt.moonlitBackground,
    icon: undefined,
    screenshots: [visualArt.moonlitBackground],
    favorite: false,
    hidden: false,
    tags: ["Mystery"],
    genres: ["Mystery"],
    metadata: {
      ...baseMetadata,
      original_name: "Moonlit",
      vndb_rating: 7.9,
      vndb_id: "v1002",
      cover: visualArt.moonlitCover,
      background: visualArt.moonlitBackground,
    },
    play_tracker: {
      ...baseTracker,
      total_seconds: 7_200,
      completion_status: "completed",
      last_played: "2026-06-09T18:00:00.000Z",
      user_rating: 8.1,
      achievements_total: 12,
      achievements_unlocked: 12,
      finished: true,
      completion_count: 1,
    },
    save_data: baseSaveData,
    aliases: [],
    tag_entries: [],
    play_time_seconds: 7_200,
    last_played: "2026-06-09T18:00:00.000Z",
    rating: 8.1,
    release_year: 2025,
  },
  {
    id: "visual-3",
    name: "Garden Protocol",
    exe_path: "",
    install_dir: undefined,
    game_type: "visual_novel",
    library_source: "manual",
    library_id: "garden",
    launch_uri: undefined,
    last_imported_at: fixedNow,
    platform: "pc",
    created_at: "2026-06-07T12:00:00.000Z",
    updated_at: fixedNow,
    description: "Uninstalled sample for metadata and status coverage.",
    cover: visualArt.gardenCover,
    background: visualArt.gardenBackground,
    icon: undefined,
    screenshots: [visualArt.gardenBackground],
    favorite: true,
    hidden: false,
    tags: ["Slice of Life"],
    genres: ["Slice of Life"],
    metadata: {
      ...baseMetadata,
      original_name: "Garden",
      vndb_rating: 8.0,
      vndb_id: "v1003",
      cover: visualArt.gardenCover,
      background: visualArt.gardenBackground,
    },
    play_tracker: {
      ...baseTracker,
      total_seconds: 0,
      completion_status: "plan_to_play",
      user_rating: 7.5,
    },
    save_data: baseSaveData,
    aliases: [],
    tag_entries: [],
    play_time_seconds: 0,
    rating: 7.5,
    release_year: 2024,
  },
  {
    id: "visual-4",
    name: "Metadata Only Sonata",
    exe_path: "D:\\Games\\Sonata\\Sonata.exe",
    install_dir: "D:\\Games\\Sonata",
    game_type: "visual_novel",
    library_source: "manual",
    library_id: "sonata",
    launch_uri: undefined,
    last_imported_at: fixedNow,
    platform: undefined,
    created_at: "2026-06-08T12:00:00.000Z",
    updated_at: fixedNow,
    description: "Canonical metadata-only item for model convergence coverage.",
    cover: undefined,
    background: undefined,
    icon: undefined,
    screenshots: [],
    favorite: false,
    hidden: false,
    tags: [],
    genres: undefined,
    metadata: {
      ...baseMetadata,
      developer: "Canonical Works",
      publisher: "Canonical Works",
      genres: ["Metadata"],
      original_name: "Sonata Canon",
      vndb_rating: 8.4,
      cover: visualArt.gardenCover,
      background: visualArt.gardenBackground,
      release_year: 2026,
    },
    play_tracker: {
      ...baseTracker,
      total_seconds: 5_400,
      completion_status: "playing",
      last_played: "2026-06-08T18:00:00.000Z",
      user_rating: 8.4,
    },
    save_data: baseSaveData,
    aliases: [],
    tag_entries: [],
    play_time_seconds: 0,
    last_played: undefined,
    rating: undefined,
    release_year: undefined,
  },
];

const largeGames = Array.from({ length: 520 }, (_, index) => {
  const source = seededGames[index % seededGames.length];
  const n = index + 1;
  return {
    ...source,
    id: `large-${n}`,
    name: `Archive ${String(n).padStart(3, "0")}`,
    favorite: index % 17 === 0,
    created_at: `2026-05-${String((index % 28) + 1).padStart(2, "0")}T12:00:00.000Z`,
    play_tracker: {
      ...source.play_tracker,
      total_seconds: (index % 40) * 1800,
      last_played: index % 3 === 0 ? `2026-06-${String((index % 10) + 1).padStart(2, "0")}T18:00:00.000Z` : undefined,
      completion_status: index % 11 === 0 ? "completed" : index % 5 === 0 ? "playing" : "not_started",
    },
    play_time_seconds: (index % 40) * 1800,
    last_played: index % 3 === 0 ? `2026-06-${String((index % 10) + 1).padStart(2, "0")}T18:00:00.000Z` : undefined,
  };
});

async function installTauriMock(page: Page, mode: MockMode) {
  await page.addInitScript(
    ({ games, activeMode, now }) => {
      const callbacks: Record<number, (...args: unknown[]) => unknown> = {};
      let callbackId = 1;
      const settings = {
        theme: "dark",
        watch_dirs: activeMode !== "empty"
          ? [
              "D:\\Games\\Very Long Visual Novel Collection\\Studio Archive\\Season One\\Localized Builds",
              "E:\\Portable\\MoeGame\\Another Long Library Path\\Fan Disc Archive\\Chinese Patches",
            ]
          : [],
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
        steam_id: undefined,
        steam_api_key: undefined,
        autostart_enabled: false,
        startup_mode: "dashboard",
      };

      window.__TAURI_INTERNALS__ = {
        callbacks,
        metadata: {
          currentWindow: { label: "main" },
          currentWebview: { label: "main" },
        },
        invoke: async (cmd: string, args?: Record<string, unknown>) => {
          switch (cmd) {
            case "get_games":
            case "search_games":
              return activeMode !== "empty" ? games : [];
            case "get_game":
              return games.find((game) => game.id === args?.id) ?? null;
            case "get_settings":
            case "update_settings":
            case "add_watch_dir":
            case "remove_watch_dir":
              return settings;
            case "cache_thumbnail":
              return { key: args?.key, source: args?.source, path: args?.source, size: 0, cached: true };
            case "get_thumbnail":
              return null;
            case "get_tasks":
              return [];
            case "get_migration_status":
              return [
                {
                  version: 1,
                  description: "Migrate legacy flat fields into GameMetadata / PlayTracker",
                  applied: true,
                },
              ];
            case "get_dashboard_data":
              return {
                total_games: games.length,
                completed_games: games.filter((game) => game.play_tracker?.completion_status === "completed").length,
                total_playtime_hours: 42,
                completion_rate: 34,
                disk_usage_gb: 12.5,
                top_tags: [
                  { name: "Visual Novel", count: games.length },
                  { name: "Story", count: 2 },
                ],
                top_developers: [{ name: "Moe Lab", count: games.length }],
                monthly_heatmap: [
                  { month: "2026-04", hours: 6 },
                  { month: "2026-05", hours: 14 },
                  { month: "2026-06", hours: 22 },
                ],
                recent_sessions: [
                  { game_name: "Asterism Script", hours: 2.5, date: now },
                ],
                collection_counts: [
                  { id: "playing", name: "Playing", count: 1 },
                  { id: "completed", name: "Completed", count: 1 },
                ],
                status_distribution: [
                  { status: "playing", count: 1 },
                  { status: "completed", count: 1 },
                  { status: "not_started", count: Math.max(0, games.length - 2) },
                ],
              };
            case "get_smart_collections":
              return [];
            case "get_recommendations":
              return [
                { name: "Asterism Script", score: 92, reasons: ["Seeded visual profile"] },
              ];
            case "scrape_games":
            case "scrape_game":
              return [
                {
                  title: "Asterism Script",
                  description: "Editorial discovery mock with cached cover art and enough text to exercise G4d rail layout.",
                  cover: games[0]?.cover,
                  background: games[0]?.background,
                  tags: ["Visual Novel", "Story"],
                  rating: 9.1,
                  release_year: 2026,
                  source: "vndb",
                  source_id: "v1001",
                  detail: {
                    developer: "Moe Lab",
                    publisher: "Moe Lab",
                    original_name: "ASTR",
                    aliases: ["Asterism Script"],
                    genres: ["Visual Novel"],
                    homepage: "https://example.invalid/asterism",
                    screenshots: [games[0]?.background],
                    languages: ["zh"],
                    engine: "KiriKiri",
                    age_rating: "全年龄",
                    series: "Visual Mock",
                    release_date: "2026-01-10",
                    vndb_id: "v1001",
                    bangumi_id: "1001",
                    dl_site_id: undefined,
                    voice_languages: ["ja"],
                  },
                },
                {
                  title: "Moonlit Archive",
                  description: "Secondary mock result for candidate row and spotlight preview selection.",
                  cover: games[1]?.cover,
                  background: games[1]?.background,
                  tags: ["Mystery"],
                  rating: 8.4,
                  release_year: 2025,
                  source: "bangumi",
                  source_id: "1002",
                  detail: {
                    developer: "Moe Lab",
                    publisher: "Moe Lab",
                    original_name: "Moonlit",
                    aliases: ["Moonlit Archive"],
                    genres: ["Mystery"],
                    homepage: "https://example.invalid/moonlit",
                    screenshots: [games[1]?.background],
                    languages: ["zh"],
                    engine: "RenPy",
                    age_rating: "全年龄",
                    series: "Visual Mock",
                    release_date: "2025-08-12",
                    vndb_id: "v1002",
                    bangumi_id: "1002",
                    dl_site_id: undefined,
                    voice_languages: ["ja"],
                  },
                },
              ];
            case "get_performance_snapshot":
              return {
                timestamp: Date.parse(now),
                game_count: games.length,
                database_size_bytes: 1024 * 1024 * 8,
                cache_size_bytes: 1024 * 512,
                target_dir_size_bytes: 1024 * 1024 * 128,
              };
            case "run_diagnostics":
              return {
                system_info: {
                  os: "Windows",
                  arch: "x64",
                  memory_gb: 32,
                  disk_free_gb: 512,
                  locale_emulator_installed: true,
                },
                app_info: {
                  version: "0.1.1",
                  database_size_mb: 8,
                  game_count: games.length,
                  scrape_sources: ["vndb", "bangumi"],
                },
                issues: [
                  {
                    severity: "Warning",
                    category: "VisualMock",
                    message: "Long diagnostic message for Aura inset wrapping and issue-list coverage across narrow layouts.",
                    solution: "Keep rows readable.",
                  },
                ],
                recommendations: ["Visual smoke mock"],
              };
            case "get_downloads":
              return [
                {
                  id: "visual-download",
                  url: "https://example.invalid/demo.zip",
                  filename: "demo-with-a-very-long-file-name-for-mobile-layout-check.zip",
                  save_path: "D:\\Downloads\\demo.zip",
                  total_size: 1000,
                  downloaded_size: 620,
                  progress: 0.62,
                  speed: 1024 * 420,
                  status: "Downloading",
                  retry_count: 0,
                  max_retries: 3,
                  auto_extract: false,
                  auto_import: false,
                  headers: {},
                },
                {
                  id: "visual-download-done",
                  url: "https://example.invalid/patch.zip",
                  filename: "patch.zip",
                  save_path: "D:\\Downloads\\patch.zip",
                  total_size: 2048,
                  downloaded_size: 2048,
                  progress: 1,
                  speed: 0,
                  status: "Completed",
                  retry_count: 0,
                  max_retries: 3,
                  auto_extract: false,
                  auto_import: false,
                  headers: {},
                },
              ];
            case "get_platform_import_status":
              return {
                steam_installed: true,
                steam_path: "C:\\Program Files (x86)\\Steam",
                steam_id: "76561198000000000",
                has_steam_api_key: true,
                steam_api_key_validated: true,
                epic_installed: false,
                epic_path: null,
                last_scan_at: now,
              };
            case "detect_save_candidates":
              return [
                {
                  path: "D:\\Games\\Asterism\\SaveData",
                  category: "Visual Novel",
                  score: 92,
                  write_count: 18,
                  last_write_time: now,
                  file_count: 24,
                  total_size_bytes: 1024 * 512,
                  matched_rule: "visual-smoke",
                },
              ];
            case "list_save_snapshots":
              return [
                {
                  id: "snapshot-1",
                  file_path: "D:\\Backups\\Asterism\\snapshot-1.zip",
                  file_name: "snapshot-1.zip",
                  created_at: now,
                  file_size_bytes: 1024 * 256,
                  note: "Visual smoke snapshot",
                  file_count: 24,
                },
                {
                  id: "snapshot-2",
                  file_path: "D:\\Backups\\Asterism\\snapshot-2.zip",
                  file_name: "snapshot-2.zip",
                  created_at: "2026-06-10T08:00:00.000Z",
                  file_size_bytes: 1024 * 128,
                  note: "Previous visual smoke snapshot",
                  file_count: 18,
                },
              ];
            case "pick_directory":
              return "D:\\Games\\Very Long Visual Novel Collection\\ROM Library\\Nintendo Switch Dumps";
            case "scan_directory_for_games":
              return { imported: 2, skipped: 1 };
            case "search_emulators":
              return [
                {
                  id: "ryujinx",
                  name: "Ryujinx",
                  install_dir: "C:\\Emulators\\Ryujinx",
                  executable: "C:\\Emulators\\Ryujinx\\Ryujinx.exe",
                  profiles: [
                    {
                      profile_name: "Switch",
                      platform_ids: ["switch"],
                      image_extensions: ["xci", "nsp"],
                      startup_arguments: "\"{ImagePath}\"",
                    },
                  ],
                },
                {
                  id: "pcsx2",
                  name: "PCSX2",
                  install_dir: "C:\\Emulators\\PCSX2",
                  executable: "C:\\Emulators\\PCSX2\\pcsx2-qt.exe",
                  profiles: [
                    {
                      profile_name: "PlayStation 2",
                      platform_ids: ["ps2"],
                      image_extensions: ["iso", "chd"],
                      startup_arguments: "\"{ImagePath}\"",
                    },
                  ],
                },
              ];
            case "scan_roms":
              return [
                {
                  path: "D:\\Games\\Very Long Visual Novel Collection\\ROM Library\\Nintendo Switch Dumps\\Asterism Script.xci",
                  filename: "Asterism Script.xci",
                  name: "Asterism Script",
                  extension: "xci",
                  size_bytes: 8_589_934_592,
                  platform: "switch",
                },
              ];
            case "migrate_from_csharp":
              return new Promise((resolve) => {
                window.setTimeout(() => {
                  resolve({
                    total_found: 4,
                    imported: 3,
                    updated: 1,
                    skipped: 0,
                    media_copied: 3,
                    media_missing: 1,
                    errors: [],
                    duration_secs: 1.2,
                    source_label: "Visual legacy library",
                    source_ids: ["legacy-1", "legacy-2", "legacy-3", "legacy-4"],
                    backup_dir: "D:\\Backups\\MoeGame",
                  });
                }, 800);
              });
            case "verify_migration_ids":
              return {
                expected_count: 4,
                actual_count: 4,
                matched_count: 4,
                missing_count: 0,
                missing_ids: [],
                count_match: true,
                with_cover: 3,
                with_background: 2,
                with_description: 4,
                cover_rate: 75,
                issues: [],
              };
            default:
              return null;
          }
        },
        convertFileSrc: (filePath: string) => filePath,
        transformCallback: (callback: (...args: unknown[]) => unknown) => {
          const id = callbackId++;
          callbacks[id] = callback;
          return id;
        },
        unregisterCallback: (id: number) => {
          delete callbacks[id];
        },
        runCallback: (id: number, args: unknown[]) => callbacks[id]?.(...args),
      };
    },
    { games: mode === "large" ? largeGames : seededGames, activeMode: mode, now: fixedNow }
  );
}

async function openApp(page: Page, mode: MockMode) {
  await installTauriMock(page, mode);
  await page.goto("/");
  await expect(page.getByTestId("app-shell")).toBeVisible();
}

async function expectNoHorizontalOverflow(page: Page, selector = ".aura-page") {
  await expect(page.locator(selector).first()).toBeVisible();
  const overflows = await page.locator(selector).first().evaluate((el) => el.scrollWidth > el.clientWidth + 1);
  expect(overflows).toBe(false);
}

test("renders the seeded Switch-style home shell", async ({ page }, testInfo: TestInfo) => {
  await openApp(page, "seeded");

  await expect(page.getByTestId("switch-home-stage")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Asterism Script" })).toBeVisible();
  await expect(page.getByRole("option", { name: /Moonlit Archive|M/ }).first()).toBeVisible();
  await expect(page.getByRole("listbox")).toBeVisible();
  await expect(page.locator(".wizard-overlay")).toHaveCount(0);

  await page.screenshot({
    path: testInfo.outputPath(`home-${testInfo.project.name}.png`),
    fullPage: false,
  });
});

test("renders the first-run empty state without a database", async ({ page }, testInfo: TestInfo) => {
  await openApp(page, "empty");

  await expect(page.getByTestId("switch-home-empty")).toBeVisible();
  await expect(page.locator(".wizard-overlay")).toBeVisible();
  await expect(page.locator(".wizard-overlay.aura-page")).toHaveAttribute("data-aura-echo", "SETUP");
  await expect(page.locator(".wizard-overlay .aura-title")).toBeVisible();
  await expect(page.locator(".wizard-overlay .wizard.aura-bevel")).toBeVisible();
  const firstRunProgress = await page.locator(".wizard-overlay .progress-track span").evaluate((el) => {
    const style = getComputedStyle(el);
    return { transform: style.transform, transformOrigin: style.transformOrigin };
  });
  expect(firstRunProgress.transform).not.toBe("none");
  expect(firstRunProgress.transformOrigin).toContain("0px");
  await expect(page.getByTestId("switch-home-stage")).toHaveCount(0);

  await page.screenshot({
    path: testInfo.outputPath(`empty-${testInfo.project.name}.png`),
    fullPage: false,
  });
});

test("scopes Aura to tool pages without leaking into Switch or Big Picture shells", async ({ page }) => {
  await openApp(page, "seeded");

  await expect(page.getByTestId("switch-home-stage")).toBeVisible();
  await expect(page.locator(".aura-page")).toHaveCount(0);

  await page.locator(".dock-btn").nth(2).click();
  await expect(page.locator(".aura-page")).toBeVisible();

  await page.locator(".bp-toggle").click();
  await expect(page.locator(".bp")).toBeVisible();
  await expect(page.locator(".aura-page")).toHaveCount(0);
});

test("renders Aura shell across dock tool pages", async ({ page }, testInfo: TestInfo) => {
  await installTauriMock(page, "seeded");

  const pages = [
    { dockIndex: 0, echo: "DISCOVERY" },
    { dockIndex: 1, echo: "SCRAPER" },
    { dockIndex: 2, echo: "DOWNLOADS" },
    { dockIndex: 3, echo: "BACKUP" },
    { dockIndex: 4, echo: "STATISTICS" },
    { dockIndex: 5, echo: "IMPORT" },
    { dockIndex: 6, echo: "EMULATOR" },
    { dockIndex: 7, echo: "DIAGNOSTICS" },
    { dockIndex: 8, echo: "SETTINGS" },
  ];

  for (const item of pages) {
    await page.goto("/");
    await expect(page.getByTestId("switch-home-stage")).toBeVisible();
    await page.locator(".dock-btn").nth(item.dockIndex).click();
    const aura = page.locator(".aura-page").first();
    await expect(aura).toBeVisible();
    await expect(aura).toHaveAttribute("data-aura-echo", item.echo);
    await expect(aura.locator(".aura-title").first()).toBeVisible();
  }

  await page.screenshot({
    path: testInfo.outputPath(`aura-pages-${testInfo.project.name}.png`),
    fullPage: false,
  });
});

test("renders G4b Aura data pages with structured controls", async ({ page }) => {
  await installTauriMock(page, "seeded");

  await page.goto("/");
  await expect(page.getByTestId("switch-home-stage")).toBeVisible();
  await page.locator(".dock-btn").nth(4).click();
  const stats = page.locator(".stats-page.aura-page");
  await expect(stats).toBeVisible();
  await expect(stats).toHaveAttribute("data-aura-echo", "STATISTICS");
  await expect(stats.getByRole("heading", { name: "统计" })).toBeVisible();
  await expect(stats.locator(".bento")).toBeVisible();
  await expect(stats.locator(".metric-card.hero")).toBeVisible();
  await expect(stats.locator(".aura-num").first()).toBeVisible();
  await expect(stats.locator(".status-fill").first()).toBeVisible();
  await expect(stats.locator(".trend-line")).toBeVisible();
  await expect(stats.locator(".hint .aura-num")).toBeVisible();
  await expect(stats.locator(".heat-cell .aura-num").first()).toBeVisible();
  const bentoColumns = await stats.locator(".bento").evaluate((el) => getComputedStyle(el).gridTemplateColumns.split(" ").map(parseFloat));
  if (page.viewportSize()!.width > 1100) {
    expect(bentoColumns[0] / bentoColumns[1]).toBeGreaterThan(1.7);
  }
  const auraDataB = await stats.evaluate((el) => getComputedStyle(el).getPropertyValue("--aura-data-b").trim());
  expect(auraDataB).toContain("232, 85, 127");
  await expectNoHorizontalOverflow(page, ".stats-page");

  await page.goto("/");
  await expect(page.getByTestId("switch-home-stage")).toBeVisible();
  await page.locator(".dock-btn").nth(2).click();
  const downloads = page.locator(".page.aura-page");
  await expect(downloads).toHaveAttribute("data-aura-echo", "DOWNLOADS");
  await expect(downloads.locator(".pill.active")).toContainText("下载中");
  await expect(downloads.locator(".pill.done")).toContainText("已完成");
  const task = downloads.locator(".task").first();
  await expect(task).toBeVisible();
  await expect(task.locator(".speed-info.aura-num")).toBeVisible();
  const barStyle = await task.locator(".bar").first().evaluate((el) => {
    const style = getComputedStyle(el);
    return { transform: style.transform, transformOrigin: style.transformOrigin };
  });
  const barTransform = barStyle.transform;
  expect(barTransform).not.toBe("none");
  expect(barStyle.transformOrigin).toContain("0px");
  const taskStyles = await task.evaluate((el) => {
    const style = getComputedStyle(el);
    return { borderRadius: style.borderRadius, backdropFilter: style.backdropFilter, borderBottomWidth: style.borderBottomWidth };
  });
  expect(parseFloat(taskStyles.borderRadius)).toBeLessThan(1);
  expect(taskStyles.backdropFilter).toBe("none");
  expect(parseFloat(taskStyles.borderBottomWidth)).toBeGreaterThan(0);
  await expectNoHorizontalOverflow(page, ".page.aura-page");

  await page.goto("/");
  await expect(page.getByTestId("switch-home-stage")).toBeVisible();
  await page.locator(".dock-btn").nth(7).click();
  const diagnostics = page.locator(".tool-page.aura-page");
  await expect(diagnostics).toHaveAttribute("data-aura-echo", "DIAGNOSTICS");
  await expect(diagnostics.locator(".stat-grid")).toBeVisible();
  await expect(diagnostics.locator(".row-list").first()).toBeVisible();
  await expect(diagnostics.locator(".data-row").first()).toBeVisible();
  await expect(diagnostics.getByText("Long diagnostic message")).toBeVisible();
  await expect(diagnostics.locator(".log-well.aura-inset")).toBeVisible();
  await expect(diagnostics.locator(".log-well code").first()).toContainText("os=");
  await expect(diagnostics.locator(".status-dot").first()).toBeVisible();
  const statusAnimation = await diagnostics.locator(".status-dot").first().evaluate((el) => getComputedStyle(el).animationName);
  expect(statusAnimation).toContain("pulse-dot");
  await expectNoHorizontalOverflow(page, ".tool-page.aura-page");
});

test("renders G4c Aura library-operation pages", async ({ page }) => {
  await installTauriMock(page, "seeded");

  await page.goto("/");
  await expect(page.getByTestId("switch-home-stage")).toBeVisible();
  await page.locator(".dock-btn").nth(8).click();
  const settings = page.locator(".settings-page.aura-page");
  await expect(settings).toHaveAttribute("data-aura-echo", "SETTINGS");
  await expect(settings.locator(".aura-bevel")).toHaveCount(1);
  await expect(settings.locator(".settings-anchors")).toBeVisible();
  await expect(settings.locator(".setting-group").first()).toBeVisible();
  const settingGroupCount = await settings.locator(".setting-group").count();
  await expect(settings.locator(".setting-group.aura-panel")).toHaveCount(settingGroupCount);
  const layoutColumns = await settings.locator(".settings-layout").evaluate((el) => getComputedStyle(el).gridTemplateColumns.split(" ").map(parseFloat));
  if (page.viewportSize()!.width > 900) {
    expect(layoutColumns[0]).toBeGreaterThan(205);
    expect(layoutColumns[0]).toBeLessThan(235);
    const anchorPosition = await settings.locator(".settings-anchors").evaluate((el) => getComputedStyle(el).position);
    expect(anchorPosition).toBe("sticky");
  } else {
    expect(layoutColumns.length).toBe(1);
    const anchorPosition = await settings.locator(".settings-anchors").evaluate((el) => getComputedStyle(el).position);
    expect(anchorPosition).toBe("static");
  }
  if (page.viewportSize()!.width <= 560) {
    const anchorColumns = await settings.locator(".settings-anchors").evaluate((el) => getComputedStyle(el).gridTemplateColumns.split(" ").length);
    expect(anchorColumns).toBe(1);
  }
  const rowBorder = await settings.locator(".setting-row").first().evaluate((el) => getComputedStyle(el).borderBottomWidth);
  expect(parseFloat(rowBorder)).toBeGreaterThan(0);
  const formBorder = await settings.locator(".form-row").nth(1).evaluate((el) => getComputedStyle(el).borderBottomWidth);
  expect(parseFloat(formBorder)).toBeGreaterThan(0);
  const dirStyle = await settings.locator(".dirs article").first().evaluate((el) => {
    const style = getComputedStyle(el);
    return { borderBottomWidth: style.borderBottomWidth, backgroundColor: style.backgroundColor };
  });
  expect(parseFloat(dirStyle.borderBottomWidth)).toBeGreaterThan(0);
  expect(dirStyle.backgroundColor).toBe("rgba(0, 0, 0, 0)");
  await expectNoHorizontalOverflow(page, ".settings-page");

  await page.goto("/");
  await expect(page.getByTestId("switch-home-stage")).toBeVisible();
  await page.locator(".dock-btn").nth(3).click();
  const backup = page.locator(".tool-page.aura-page");
  await expect(backup).toHaveAttribute("data-aura-echo", "BACKUP");
  await expect(backup.locator(".timeline-list")).toBeVisible();
  await expect(backup.locator(".timeline-row").first()).toBeVisible();
  await expect(backup.locator(".timeline-node").first()).toBeVisible();
  const axis = await backup.locator(".timeline-list").evaluate((el) => {
    const style = getComputedStyle(el, "::before");
    return { width: style.width, backgroundColor: style.backgroundColor };
  });
  expect(parseFloat(axis.width)).toBe(1);
  expect(axis.backgroundColor).not.toBe("rgba(0, 0, 0, 0)");
  const timelineGrid = await backup.locator(".timeline-row").first().evaluate((el) => getComputedStyle(el).gridTemplateColumns.split(" ").map(parseFloat));
  expect(timelineGrid[0]).toBeLessThan(30);
  await expect(backup.locator(".timeline-row .row-action").first()).toBeVisible();
  const timelineRowStyle = await backup.locator(".timeline-row").first().evaluate((el) => {
    const style = getComputedStyle(el);
    return { borderRadius: style.borderRadius, boxShadow: style.boxShadow, borderBottomWidth: style.borderBottomWidth };
  });
  expect(parseFloat(timelineRowStyle.borderRadius)).toBeLessThan(1);
  expect(timelineRowStyle.boxShadow).toBe("none");
  expect(parseFloat(timelineRowStyle.borderBottomWidth)).toBeGreaterThan(0);
  const actionBox = await backup.locator(".timeline-row .row-action").first().boundingBox();
  const copyBox = await backup.locator(".timeline-row .timeline-copy").first().boundingBox();
  expect(actionBox!.x).toBeGreaterThanOrEqual(copyBox!.x - 1);
  await expect(backup.locator(".data-row .row-action").first()).toBeVisible();
  await expectNoHorizontalOverflow(page, ".tool-page.aura-page");
});

test("renders G4d Aura content and onboarding pages", async ({ page }) => {
  await installTauriMock(page, "seeded");

  await page.goto("/");
  await expect(page.getByTestId("switch-home-stage")).toBeVisible();
  await page.locator(".dock-btn").nth(0).click();
  const discovery = page.locator(".page.aura-page[data-aura-echo='DISCOVERY']");
  await expect(discovery).toBeVisible();
  await expect(discovery.locator(".sakura-layer span")).toHaveCount(8);
  await discovery.locator(".search-box input").fill("Asterism");
  await discovery.locator(".btn-search").click();
  await expect(discovery.locator(".editorial-rail")).toBeVisible();
  await expect(discovery.locator(".editorial-card").first()).toBeVisible();
  await expect(discovery.locator(".editorial-card .cached-image").first()).toBeVisible();
  const railMetrics = await discovery.locator(".editorial-rail").evaluate((el) => {
    const style = getComputedStyle(el);
    const parent = el.parentElement!;
    return {
      display: style.display,
      overflowX: style.overflowX,
      fitsParent: el.getBoundingClientRect().right <= parent.getBoundingClientRect().right + 1,
    };
  });
  expect(railMetrics.display).toBe("flex");
  expect(railMetrics.overflowX).toBe("auto");
  expect(railMetrics.fitsParent).toBe(true);

  await page.goto("/");
  await expect(page.getByTestId("switch-home-stage")).toBeVisible();
  await page.locator(".dock-btn").nth(1).click();
  const scraper = page.locator(".page.aura-page[data-aura-echo='SCRAPER']");
  await expect(scraper).toBeVisible();
  await scraper.locator(".toolbar input").fill("Asterism");
  await scraper.locator("button.primary").click();
  await expect(scraper.locator(".candidate-row").first()).toBeVisible();
  await expect(scraper.locator(".candidate-row .match").first()).toContainText("%");
  await expect(scraper.locator(".preview-panel.aura-panel--spot")).toBeVisible();
  await expect(scraper.locator(".preview-card .match")).toContainText("%");
  await expectNoHorizontalOverflow(page, ".page.aura-page");

  await page.goto("/");
  await expect(page.getByTestId("switch-home-stage")).toBeVisible();
  await page.locator(".dock-btn").nth(6).click();
  const emulator = page.locator(".overlay.aura-page[data-aura-echo='EMULATOR']");
  await expect(emulator).toBeVisible();
  await expect(emulator.locator(".dialog.aura-panel.aura-bevel")).toBeVisible();
  await expect(emulator.locator(".aura-empty.aura-inset").first()).toBeVisible();
  await emulator.locator(".btn-row button").first().click();
  await expect(emulator.locator(".emu-row").first()).toBeVisible();
  const emuRowStyle = await emulator.locator(".emu-row").first().evaluate((el) => {
    const style = getComputedStyle(el);
    return { borderBottomWidth: style.borderBottomWidth, borderRadius: style.borderRadius };
  });
  expect(parseFloat(emuRowStyle.borderBottomWidth)).toBeGreaterThan(0);
  expect(parseFloat(emuRowStyle.borderRadius)).toBeLessThan(1);
});

test("renders G4d first-run migration progress", async ({ page }) => {
  await openApp(page, "empty");

  const wizard = page.locator(".wizard-overlay.aura-page");
  await expect(wizard).toBeVisible();
  await expect(wizard.locator(".wizard.aura-bevel")).toBeVisible();
  await wizard.locator(".actions .btn-primary").click();
  await wizard.locator(".actions .btn-primary").click();
  await wizard.locator(".entry-card").nth(2).click();

  const migration = page.locator(".overlay.aura-page[data-aura-echo='MIGRATION']");
  await expect(migration).toBeVisible();
  await expect(migration.locator(".dialog.aura-panel.aura-bevel")).toBeVisible();
  if ((await migration.locator(".path").count()) === 0) {
    await migration.locator(".btn.primary").first().click();
  }
  await expect(migration.locator(".path")).toBeVisible();
  await migration.locator(".actions .btn.primary").click();
  await expect(migration.locator(".migration-progress .fill")).toBeVisible();
  const migrationProgress = await migration.locator(".migration-progress .fill").evaluate((el) => {
    const style = getComputedStyle(el);
    return { transform: style.transform, transformOrigin: style.transformOrigin };
  });
  expect(migrationProgress.transform).not.toBe("none");
  expect(migrationProgress.transformOrigin).toContain("0px");
  await expect(migration.locator(".report.aura-card").first()).toBeVisible();
  await expect(migration.locator(".report.glass-card")).toHaveCount(0);
});

test("keeps the all-games grid context after opening detail", async ({ page }, testInfo: TestInfo) => {
  await openApp(page, "seeded");

  await page.getByRole("button", { name: "全部游戏" }).click();
  await expect(page.getByTestId("all-games-grid")).toBeVisible();

  await page.locator(".game-card").filter({ hasText: "Asterism Script" }).first().click();
  await expect(page.getByTestId("game-detail-page")).toBeVisible();

  await page.getByRole("button", { name: /返回游戏库/ }).click();
  await expect(page.getByTestId("all-games-grid")).toBeVisible();
  await expect(page.getByTestId("switch-home-stage")).toHaveCount(0);

  await page.screenshot({
    path: testInfo.outputPath(`all-grid-return-${testInfo.project.name}.png`),
    fullPage: false,
  });
});

test("renders metadata-only games without legacy field fallbacks", async ({ page }) => {
  await openApp(page, "seeded");

  await page.getByRole("button", { name: "全部游戏" }).click();
  const card = page.locator(".game-card").filter({ hasText: "Metadata Only Sonata" }).first();
  await expect(card).toBeVisible();
  await expect(card).toContainText("Canonical Works");
  await expect(card).toContainText("Metadata");
  await expect(card.locator("img.cached-image")).toHaveAttribute("alt", "Metadata Only Sonata");

  await card.click();
  const detail = page.getByTestId("game-detail-page");
  await expect(detail).toBeVisible();
  await expect(detail.getByRole("heading", { name: "Metadata Only Sonata" })).toBeVisible();
  await expect(detail.getByText("Canonical Works").first()).toBeVisible();
  await expect(detail.getByText("Sonata Canon")).toBeVisible();
  await expect(detail.getByText("8.4")).toBeVisible();
});

test("windows the all-games grid for large libraries", async ({ page }) => {
  await openApp(page, "large");

  await page.getByRole("button", { name: "全部游戏" }).click();
  await expect(page.getByTestId("all-games-grid")).toBeVisible();
  await expect(page.getByText(/520 \/ 520 款/)).toBeVisible();

  const renderedCards = await page.locator(".game-card").count();
  expect(renderedCards).toBeGreaterThan(0);
  expect(renderedCards).toBeLessThan(120);
});

test("renders list mode as a single readable column", async ({ page }) => {
  await openApp(page, "large");

  await page.getByRole("button", { name: "全部游戏" }).click();
  await page.getByRole("button", { name: "列表" }).click();

  await expect(page.getByRole("heading", { name: "列表视图" })).toBeVisible();
  const cards = page.locator(".game-card");
  await expect(cards.first()).toBeVisible();

  const [first, second] = await Promise.all([
    cards.nth(0).boundingBox(),
    cards.nth(1).boundingBox(),
  ]);
  expect(first).not.toBeNull();
  expect(second).not.toBeNull();
  expect(Math.abs((first?.x ?? 0) - (second?.x ?? 0))).toBeLessThan(2);
  expect(first?.width ?? 0).toBeGreaterThan(page.viewportSize()!.width * 0.78);
});
