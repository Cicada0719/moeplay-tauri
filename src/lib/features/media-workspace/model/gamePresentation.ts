import type { Game } from "../../../api";
import {
  coverOf,
  developerOf,
  gameCompletionStatus,
  gameLastPlayed,
  gameRating,
  gameTotalSeconds,
  heroImageOf,
  isInstalled,
  originalNameOf,
  platformOf,
  publisherOf,
  releaseYearOf,
  screenshotsOf,
  tagsOf,
} from "../../../utils/game";
import { fileSrc } from "../../../utils";
import type {
  MediaAssetQuality,
  MediaPresentationAction,
  MediaPresentationItem,
  PresentationAsset,
} from "./types";

export interface GamePresentationActions {
  open?: (game: Game) => void | Promise<void>;
  select?: (gameId: string) => void;
  launch?: (gameId: string) => void | Promise<void>;
  toggleFavorite?: (gameId: string) => void | Promise<void>;
}

function cleanSource(source: string | null | undefined): string | undefined {
  const value = source?.trim();
  if (!value) return undefined;
  try {
    return fileSrc(value) || undefined;
  } catch {
    return value;
  }
}

function asset(id: string, src: string, role: PresentationAsset["role"], title: string): PresentationAsset {
  return {
    id,
    src,
    role,
    alt: role === "cover" ? `${title} 封面` : role === "hero" ? `${title} 主视觉` : `${title} 截图`,
    aspect: role === "cover" ? "portrait" : "landscape",
  };
}

function mediaQuality(cover: string | undefined, hero: string | undefined, screenshots: string[]): MediaAssetQuality {
  const distinctSupport = new Set(
    [hero, ...screenshots].filter((value): value is string => Boolean(value) && value !== cover),
  );
  if (cover && hero && hero !== cover && distinctSupport.size >= 2) return "a";
  if (cover && distinctSupport.size >= 1) return "b";
  if (cover || hero || screenshots.length > 0) return "c";
  return "d";
}

function buildActions(game: Game, handlers: GamePresentationActions): MediaPresentationAction[] {
  const actions: MediaPresentationAction[] = [];
  if (handlers.open) {
    actions.push({
      id: "open",
      label: "查看详情",
      emphasis: "primary",
      enabled: true,
      run: () => handlers.open?.(game),
    });
  }
  if (handlers.launch) {
    const launchable = Boolean(game.exe_path?.trim() || game.launch_uri?.trim());
    actions.push({
      id: "launch",
      label: gameTotalSeconds(game) > 0 ? "继续游玩" : "开始游戏",
      emphasis: handlers.open ? "secondary" : "primary",
      enabled: launchable,
      run: () => launchable ? handlers.launch?.(game.id) : undefined,
    });
  }
  if (handlers.toggleFavorite) {
    actions.push({
      id: "toggle-favorite",
      label: game.favorite ? "取消收藏" : "收藏",
      emphasis: "quiet",
      enabled: true,
      active: game.favorite,
      run: () => handlers.toggleFavorite?.(game.id),
    });
  }
  if (handlers.select) {
    actions.push({
      id: "select",
      label: "选择",
      emphasis: "quiet",
      enabled: true,
      run: () => handlers.select?.(game.id),
    });
  }
  return actions;
}

/** Pure adapter used by all game workspace modes. */
export function adaptGameToPresentation(
  game: Game,
  handlers: GamePresentationActions = {},
): MediaPresentationItem {
  const title = game.name.trim() || "未命名游戏";
  const coverSource = cleanSource(coverOf(game));
  const heroSource = cleanSource(heroImageOf(game));
  const screenshotSources = [...new Set(screenshotsOf(game).map(cleanSource).filter((src): src is string => Boolean(src)))]
    .filter(src => src !== coverSource && src !== heroSource);

  const cover = coverSource ? asset(`${game.id}:cover`, coverSource, "cover", title) : undefined;
  const hero = heroSource ? asset(`${game.id}:hero`, heroSource, "hero", title) : undefined;
  const screenshots = screenshotSources.map((src, index) => asset(`${game.id}:screenshot:${index}`, src, "screenshot", title));
  const media = [cover, hero, ...screenshots].filter((item): item is PresentationAsset => Boolean(item));
  const originalTitle = originalNameOf(game) ?? undefined;
  const developer = developerOf(game) || undefined;
  const year = releaseYearOf(game);

  return {
    id: game.id,
    module: "games",
    title,
    originalTitle: originalTitle && originalTitle !== title ? originalTitle : undefined,
    subtitle: [developer, year].filter(Boolean).join(" · ") || undefined,
    description: game.description?.trim() || undefined,
    cover,
    hero,
    screenshots,
    media,
    mediaQuality: mediaQuality(coverSource, heroSource, screenshotSources),
    favorite: game.favorite,
    installed: isInstalled(game),
    metadata: {
      developer,
      publisher: publisherOf(game) || undefined,
      platform: platformOf(game) || undefined,
      releaseYear: year ?? undefined,
      completionStatus: gameCompletionStatus(game),
      totalSeconds: gameTotalSeconds(game),
      lastPlayed: gameLastPlayed(game) ?? undefined,
      rating: gameRating(game) || undefined,
      tags: tagsOf(game),
    },
    actions: buildActions(game, handlers),
  };
}

export function adaptGamesToPresentation(
  games: readonly Game[],
  handlers: GamePresentationActions = {},
): MediaPresentationItem[] {
  return games.map(game => adaptGameToPresentation(game, handlers));
}

