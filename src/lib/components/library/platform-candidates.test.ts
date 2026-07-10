import { describe, expect, it } from "vitest";
import { platformCandidatesToPreviewRequest, steamSessionGamesToPreviewRequest } from "./platform-candidates";

describe("platform candidate conversion", () => {
  it("converts platform and directory data into a Library v2 preview request", () => {
    const request = platformCandidatesToPreviewRequest("steam", [{
      source: "steam",
      library_id: "480",
      name: "Spacewar",
      install_dir: "D:/Steam/steamapps/common/Spacewar",
      launch_uri: "steam://rungameid/480",
      cover_url: "https://example.test/cover.jpg",
      icon_url: null,
      store_url: "https://store.steampowered.com/app/480",
      playtime_minutes: 10,
      last_played: null,
      achievements_total: null,
      achievements_unlocked: null,
      installed: true,
      selected: true,
      skip_reason: null,
    }]);

    expect(request.source).toBe("steam");
    expect(request.records[0]).toMatchObject({
      sourceRecordId: "steam:480",
      title: "Spacewar",
      installDir: "D:/Steam/steamapps/common/Spacewar",
      platformId: { source: "steam", id: "480" },
      launchUri: "steam://rungameid/480",
      launchPath: null,
    });
    expect(request.records[0].fields["metadata.cover"]).toBe("https://example.test/cover.jpg");
  });

  it("converts Steam login-session candidates without performing a write", () => {
    const request = steamSessionGamesToPreviewRequest([{ appid: 10, name: "Counter-Strike", playtime_forever: 5, last_played: 0 }]);
    expect(request.records[0].platformId).toEqual({ source: "steam", id: "10" });
    expect(request.records[0].launchUri).toBe("steam://rungameid/10");
  });
});
