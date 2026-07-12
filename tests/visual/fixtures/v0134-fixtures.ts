import { MOCK_GAMES, MOCK_SETTINGS, type MockAppState } from "./mock-app-state";

const svg = (label: string, color: string) =>
  `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 9"><rect width="16" height="9" fill="${color}"/><text x="1" y="5" fill="white" font-size="1.2">${label}</text></svg>`)}`;

const visualGame = {
  ...MOCK_GAMES[0],
  id: "visual-fixture-owner",
  name: "Visual 五槽样本",
  screenshots: [svg("S1", "#553344"), svg("S2", "#334455"), svg("S3", "#445533")],
  metadata: {
    ...MOCK_GAMES[0].metadata,
    cover: svg("COVER", "#1f2937"),
    background: svg("HERO", "#7f1d1d"),
  },
  updated_at: "2026-07-11T10:00:00.000Z",
  play_tracker: { ...MOCK_GAMES[0].play_tracker, last_played: "2026-07-11T10:00:00.000Z" },
};

const adjacentGame = {
  ...MOCK_GAMES[1],
  id: "scene-adjacent-owner",
  name: "Scene 相邻样本",
  screenshots: [svg("ADJ-S1", "#263547"), svg("ADJ-S2", "#472635")],
  metadata: {
    ...MOCK_GAMES[1].metadata,
    cover: svg("ADJ-COVER", "#243244"),
    background: svg("ADJ-HERO", "#3f2a52"),
  },
  play_tracker: { ...MOCK_GAMES[1].play_tracker, last_played: "2026-07-10T10:00:00.000Z" },
};

export const MEDIA_WORKSPACE_APP_STATE: MockAppState = {
  settings: { ...MOCK_SETTINGS },
  games: [visualGame, adjacentGame, ...MOCK_GAMES.slice(2)],
  localStorage: {
    "moegame-startup-migrated-v1": "1",
    "moeplay-theme": "dark",
  },
};
