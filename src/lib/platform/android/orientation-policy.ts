export type OrientationMode = "auto" | "portrait" | "landscape";

export interface OrientationPolicyState {
  preferred: OrientationMode;
  temporary: OrientationMode | null;
  videoAutoLandscape: boolean;
  /** Media pages hold landscape independently from video fullscreen. */
  mediaLandscape?: boolean;
  /** Reference count so nested media/player surfaces cannot restore early. */
  mediaLandscapeCount?: number;
}

export function enterVideoFullscreen(state: OrientationPolicyState): OrientationPolicyState {
  if (!state.videoAutoLandscape || state.temporary === "landscape") return state;
  return { ...state, temporary: "landscape" };
}

export function exitVideoFullscreen(state: OrientationPolicyState): OrientationPolicyState {
  if (state.temporary === null) return state;
  return { ...state, temporary: null };
}

export function enterMediaLandscape(state: OrientationPolicyState): OrientationPolicyState {
  const count = state.mediaLandscapeCount ?? (state.mediaLandscape ? 1 : 0);
  return { ...state, mediaLandscape: true, mediaLandscapeCount: count + 1 };
}

export function exitMediaLandscape(state: OrientationPolicyState): OrientationPolicyState {
  const count = Math.max(0, (state.mediaLandscapeCount ?? (state.mediaLandscape ? 1 : 0)) - 1);
  return { ...state, mediaLandscape: count > 0, mediaLandscapeCount: count };
}

export function effectiveOrientation(state: OrientationPolicyState): OrientationMode {
  return state.temporary ?? ((state.mediaLandscape || (state.mediaLandscapeCount ?? 0) > 0) ? "landscape" : state.preferred);
}
