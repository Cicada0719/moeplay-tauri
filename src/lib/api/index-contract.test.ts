import { describe, expect, it } from "vitest";
import * as api from "./index";

// API 桶完整性契约：api/index.ts 拆分后仍须完整重导出各域模块的关键函数。
// 未来任何按域重构或误删重导出，都能被此测试与类型检查兜住。
describe("api index barrel contract", () => {
  it("games 域", () => {
    expect(typeof api.getGames).toBe("function");
    expect(typeof api.updateGameMetadata).toBe("function");
    expect(typeof api.updatePlayTracker).toBe("function");
    expect(typeof api.launchGame).toBe("function");
  });
  it("scraper / downloads 域", () => {
    expect(typeof api.scrapeGame).toBe("function");
    expect(typeof api.buildSourceUrl).toBe("function");
    expect(typeof api.downloadStart).toBe("function");
    expect(typeof api.animeGetDownloads).toBe("function");
    expect(typeof api.searchGameDownloads).toBe("function");
  });
  it("saves / metadata / settings / dashboard 域", () => {
    expect(typeof api.restoreSaveSnapshot).toBe("function");
    expect(typeof api.classifyNsfwGame).toBe("function");
    expect(typeof api.updateSettings).toBe("function");
    expect(typeof api.getDashboardData).toBe("function");
  });
  it("tasks / format / platformImport / emulators / system 域", () => {
    expect(typeof api.runDiagnostics).toBe("function");
    expect(typeof api.formatPlayTime).toBe("function");
    expect(typeof api.steamFetchAndImport).toBe("function");
    expect(typeof api.searchEmulators).toBe("function");
    expect(typeof api.getWallpaperAttribution).toBe("function");
  });
  it("secrets 重导出保留", () => {
    expect(typeof api.secretSet).toBe("function");
    expect(typeof api.secretStatus).toBe("function");
  });
  it("类型重导出仍然可用（TypeScript 编译期保证；此处显式引用一次）", () => {
    // 仅保证 ./types 的类型桶可用；值无法在运行时引用，占位以说明意图
    expect(Object.keys(api).length).toBeGreaterThan(100);
  });
});
