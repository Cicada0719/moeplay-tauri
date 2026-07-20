import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Game } from "../../stores/games.svelte";

const mocks = vi.hoisted(() => ({
  games: [] as unknown[],
  launch: vi.fn(async () => undefined),
  selectGame: vi.fn(),
  navigateTo: vi.fn(),
  notify: vi.fn(),
}));

vi.mock("../../stores/games.svelte", () => ({
  gameStore: {
    get games() { return mocks.games; },
    launch: mocks.launch,
    selectGame: mocks.selectGame,
  },
}));

vi.mock("../../stores/router.svelte", () => ({
  navigateTo: mocks.navigateTo,
}));

vi.mock("../../stores/ui.svelte", () => ({
  uiStore: { notify: mocks.notify },
}));

vi.mock("../../stores/i18n.svelte", () => {
  const dict: Record<string, string> = {
    "home.continue_hero.aria": "继续游玩",
    "home.continue_hero.title": "继续游玩",
    "home.continue_hero.count": "{count} 款",
    "home.continue_hero.launch": "继续游玩",
    "home.continue_hero.open_profile": "打开档案",
    "home.continue_hero.launching": "正在启动 {name}...",
    "home.continue_hero.last_session": "最近",
    "home.continue_hero.total_playtime": "总时长",
    "home.continue_hero.achievements": "成就",
    "home.continue_hero.achievements_aria": "成就进度 {percent}%",
  };
  return {
    i18n: {
      lang: "zh",
      locale: "zh-CN",
      t: (key: string, params?: Record<string, string | number>) => {
        let text = dict[key] ?? key;
        if (params) for (const [k, v] of Object.entries(params)) text = text.replace(`{${k}}`, String(v));
        return text;
      },
    },
  };
});

vi.mock("./useGamepad.svelte", () => ({ attachGamepad: () => Object.assign(() => {}, {}) }));

import ContinueHeroRail from "./ContinueHeroRail.svelte";

function playedGame(id: string, name: string, startTime: string): Game {
  return {
    id,
    name,
    screenshots: [],
    favorite: false,
    hidden: false,
    tags: [],
    metadata: {
      developer: "Fixture Studio",
      cover: `https://img.example.com/${id}-cover.jpg`,
      background: `https://img.example.com/${id}-hero.jpg`,
    },
    play_tracker: {
      total_seconds: 7260,
      sessions: [{ id: `${id}-s1`, start_time: startTime, duration_seconds: 3660 }],
      completion_status: "playing",
      achievements_total: 20,
      achievements_unlocked: 8,
    },
    save_data: {},
    aliases: [],
    tag_entries: [],
  } as unknown as Game;
}

beforeEach(() => {
  vi.clearAllMocks();
  window.localStorage.clear();
  mocks.games = [];
});

describe("ContinueHeroRail 静态契约", () => {
  it("走封面管线与主题 token，且 reduced-motion 双写降级", () => {
    const source = readFileSync(resolve("src/lib/components/switch/ContinueHeroRail.svelte"), "utf8");

    expect(source).toContain("CachedImage");
    expect(source).toContain('loading="lazy"');
    expect(source).toContain("heroImageOf");
    expect(source).toContain("coverOf");
    expect(source).toContain("gameStore.launch");
    expect(source).toContain("scroll-snap-type");
    expect(source).toContain("@media (prefers-reduced-motion: reduce)");
    expect(source).toContain('[data-motion="reduce"]');
    // 主题 token 消费：不允许出现硬编码 hex 调色
    expect(source).not.toMatch(/#[0-9a-fA-F]{3,8}\b/);

    const home = readFileSync(resolve("src/lib/components/switch/SwitchHome.svelte"), "utf8");
    expect(home).toContain('import ContinueHeroRail from "./ContinueHeroRail.svelte"');
    expect(home).toContain("<ContinueHeroRail />");

    const appCss = readFileSync(resolve("src/app.css"), "utf8");
    expect(appCss).toContain('.app-container[data-workspace-focus="true"][data-workspace-focus-view="home"] :is(.continue-hero');
  });
});

describe("ContinueHeroRail 渲染行为", () => {
  it("无任何会话数据时整栏不渲染（连容器都不输出）", () => {
    mocks.games = [
      { id: "game-x", name: "无记录", metadata: {}, play_tracker: { sessions: [] }, screenshots: [], tags: [], save_data: {} } as unknown as Game,
      { id: "game-y", name: "缺 tracker", metadata: {}, screenshots: [], tags: [], save_data: {} } as unknown as Game,
    ];

    const { container } = render(ContinueHeroRail);

    expect(container.querySelector("[data-testid='continue-hero-rail']")).toBeNull();
    expect(container.querySelector("section")).toBeNull();
    expect(container.textContent?.trim() ?? "").toBe("");
    expect(mocks.launch).not.toHaveBeenCalled();
  });

  it("按最近会话降序渲染，hero/cover 走 CachedImage lazy，继续按钮调用 gameStore.launch", async () => {
    mocks.games = [
      playedGame("game-old", "夏日列车", "2026-07-10T12:00:00.000Z"),
      playedGame("game-new", "星海回声", "2026-07-15T12:00:00.000Z"),
    ];

    render(ContinueHeroRail);

    const titles = screen.getAllByRole("heading", { level: 3 }).map((node) => node.textContent);
    expect(titles).toEqual(["星海回声", "夏日列车"]);

    const images = document.querySelectorAll('img[loading="lazy"]');
    expect(images.length).toBeGreaterThan(0);

    await fireEvent.click(screen.getAllByRole("button", { name: /^继续游玩$/ })[0]);
    expect(mocks.launch).toHaveBeenCalledWith("game-new");

    await fireEvent.click(screen.getAllByRole("button", { name: "打开档案" })[0]);
    expect(mocks.selectGame).toHaveBeenCalledWith("game-new");
    expect(mocks.navigateTo).toHaveBeenCalledWith("game-detail", { entity: { kind: "game", id: "game-new" }, focus: "start" });

    expect(screen.getAllByRole("progressbar", { name: "成就进度 40%" })).toHaveLength(2);
  });

  it("可折叠继续游玩区域并跨重载保留选择", async () => {
    mocks.games = [playedGame("game-1", "星海回声", "2026-07-15T12:00:00.000Z")];

    const first = render(ContinueHeroRail);
    const section = screen.getByTestId("continue-hero-rail");
    const collapse = screen.getByRole("button", { name: "隐藏继续游玩" });
    expect(collapse).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("list")).toBeInTheDocument();

    await fireEvent.click(collapse);
    expect(section).toHaveAttribute("data-collapsed", "true");
    expect(screen.queryByRole("list")).not.toBeInTheDocument();
    expect(window.localStorage.getItem("moeplay:continue-hero-collapsed")).toBe("true");
    first.unmount();

    render(ContinueHeroRail);
    const expand = screen.getByRole("button", { name: "显示继续游玩" });
    expect(screen.getByTestId("continue-hero-rail")).toHaveAttribute("data-collapsed", "true");
    expect(expand).toHaveAttribute("aria-expanded", "false");
    await fireEvent.click(expand);
    expect(screen.getByRole("list")).toBeInTheDocument();
    expect(window.localStorage.getItem("moeplay:continue-hero-collapsed")).toBe("false");
  });

  it("roving tabindex：左右方向键在主按钮间移动真实焦点", async () => {
    mocks.games = [
      playedGame("game-1", "星海回声", "2026-07-15T12:00:00.000Z"),
      playedGame("game-2", "夏日列车", "2026-07-10T12:00:00.000Z"),
    ];

    render(ContinueHeroRail);

    const primaryButtons = screen.getAllByRole("button", { name: /^继续游玩$/ });
    expect(primaryButtons[0]).toHaveAttribute("tabindex", "0");
    expect(primaryButtons[1]).toHaveAttribute("tabindex", "-1");

    const rail = screen.getByRole("list");
    await fireEvent.keyDown(rail, { key: "ArrowRight" });
    await waitFor(() => {
      expect(primaryButtons[0]).toHaveAttribute("tabindex", "-1");
      expect(primaryButtons[1]).toHaveAttribute("tabindex", "0");
    });
    await waitFor(() => expect(document.activeElement).toBe(primaryButtons[1]));

    await fireEvent.keyDown(rail, { key: "ArrowLeft" });
    await waitFor(() => expect(primaryButtons[0]).toHaveAttribute("tabindex", "0"));
  });
});
