import type { ConceptMediaAsset } from "../contracts";

export type ConceptTone = ConceptMediaAsset["tone"];

export interface TonePresentation {
  toneClass: string;
  navigationClass: string;
  colorScheme: "light" | "dark";
}

export function getTonePresentation(tone: ConceptTone): TonePresentation {
  if (tone === "light") {
    return { toneClass: "concept-tone--light", navigationClass: "concept-nav--dark-ink", colorScheme: "light" };
  }
  if (tone === "mixed") {
    return { toneClass: "concept-tone--mixed", navigationClass: "concept-nav--adaptive", colorScheme: "dark" };
  }
  return { toneClass: "concept-tone--dark", navigationClass: "concept-nav--light-ink", colorScheme: "dark" };
}

export function toneClass(tone: ConceptTone): string {
  const presentation = getTonePresentation(tone);
  return `${presentation.toneClass} ${presentation.navigationClass}`;
}
