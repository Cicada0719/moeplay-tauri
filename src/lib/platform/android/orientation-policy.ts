export type OrientationMode = "auto" | "portrait" | "landscape";

export interface OrientationPolicyState {
  preferred: OrientationMode;
  temporary: OrientationMode | null;
  videoAutoLandscape: boolean;
}

export function enterVideoFullscreen(state: OrientationPolicyState): OrientationPolicyState {
  if (!state.videoAutoLandscape || state.temporary === "landscape") return state;
  return { ...state, temporary: "landscape" };
}

export function exitVideoFullscreen(state: OrientationPolicyState): OrientationPolicyState {
  if (state.temporary === null) return state;
  return { ...state, temporary: null };
}

export function effectiveOrientation(state: OrientationPolicyState): OrientationMode {
  return state.temporary ?? state.preferred;
}
