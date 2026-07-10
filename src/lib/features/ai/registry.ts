import type { LibraryCleanupInput, ResourceFilterKind } from "./contracts";
import {
  parseJsonDocument,
  validateLibraryCleanup,
  validateNaturalLanguageFilter,
  type ValidatedLibraryChangeSet,
  type ValidatedNaturalLanguageFilter,
} from "./schema";
import { AiGatewayError } from "./contracts";

export type AiUseCase = "library_cleanup" | "natural_language_filter";

export interface OutputSchemaDefinition {
  id: string;
  version: string;
  schema: Record<string, unknown>;
}

export interface PromptDefinition {
  id: string;
  version: string;
  useCase: AiUseCase;
  systemTemplate: string;
  outputSchema: OutputSchemaDefinition;
  maxOutputTokens: number;
  timeoutMs: number;
  maxRepairAttempts: number;
  privacyFields: string[];
}

const prompts: PromptDefinition[] = [
  {
    id: "library_cleanup",
    version: "1.0.0",
    useCase: "library_cleanup",
    systemTemplate: "Return only the registered library cleanup JSON change-set. Use only supplied game IDs and whitelisted fields. Never claim a write was applied.",
    outputSchema: {
      id: "library_cleanup.change_set",
      version: "1.0.0",
      schema: { type: "object", additionalProperties: false, required: ["summary", "confidence", "operations"] },
    },
    maxOutputTokens: 2400,
    timeoutMs: 45_000,
    maxRepairAttempts: 1,
    privacyFields: ["title", "description", "tags", "selectedPublicMetadata"],
  },
  {
    id: "natural_language_filter",
    version: "1.0.0",
    useCase: "natural_language_filter",
    systemTemplate: "Compile the request into the registered whitelist-only filter DSL. Return JSON only. Never emit SQL, scripts, paths, or commands.",
    outputSchema: {
      id: "natural_language_filter.dsl",
      version: "1.0.0",
      schema: { type: "object", additionalProperties: false, required: ["kind", "filters", "sort", "explanation"] },
    },
    maxOutputTokens: 1000,
    timeoutMs: 20_000,
    maxRepairAttempts: 1,
    privacyFields: ["naturalLanguageQuery"],
  },
];

export class PromptRegistry {
  list(): PromptDefinition[] {
    return prompts.map((prompt) => structuredClone(prompt));
  }

  get(id: string, version: string): PromptDefinition {
    const prompt = prompts.find((candidate) => candidate.id === id && candidate.version === version);
    if (!prompt) {
      throw new AiGatewayError("not_configured", "requested AI prompt/schema version is not registered");
    }
    return structuredClone(prompt);
  }

  validateLibraryCleanupResponse(
    version: string,
    content: string,
    input: LibraryCleanupInput,
  ): ValidatedLibraryChangeSet {
    this.get("library_cleanup", version);
    return validateLibraryCleanup(parseJsonDocument(content), input);
  }

  validateNaturalLanguageFilterResponse(
    version: string,
    content: string,
    expectedKind: ResourceFilterKind,
  ): ValidatedNaturalLanguageFilter {
    this.get("natural_language_filter", version);
    return validateNaturalLanguageFilter(parseJsonDocument(content), expectedKind);
  }
}
