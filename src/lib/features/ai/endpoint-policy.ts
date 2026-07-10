import { AiGatewayError, type AiProviderKind } from "./contracts";

export type EndpointScope = "loopback" | "remote";

export interface ValidatedEndpoint {
  url: string;
  origin: string;
  scope: EndpointScope;
}

function isLocalKind(kind: AiProviderKind): boolean {
  return kind === "ollama" || kind === "mock";
}

function isLoopback(hostname: string): boolean {
  const normalized = hostname.toLowerCase().replace(/^\[|\]$/g, "");
  if (normalized === "localhost" || normalized === "::1") return true;
  const match = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/.exec(normalized);
  if (!match) return false;
  const octets = match.slice(1).map(Number);
  return octets.every((octet) => octet >= 0 && octet <= 255) && octets[0] === 127;
}

export function validateEndpoint(raw: string, kind: AiProviderKind): ValidatedEndpoint {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    throw new AiGatewayError("policy_rejected", "AI endpoint is not a valid absolute URL");
  }
  if (url.username || url.password) {
    throw new AiGatewayError("policy_rejected", "AI endpoint URL credentials are forbidden");
  }
  if (url.search) {
    throw new AiGatewayError("policy_rejected", "AI endpoint query parameters are forbidden");
  }
  if (url.hash) {
    throw new AiGatewayError("policy_rejected", "AI endpoint fragments are forbidden");
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new AiGatewayError("policy_rejected", "AI endpoint must use HTTP or HTTPS");
  }

  const scope: EndpointScope = isLoopback(url.hostname) ? "loopback" : "remote";
  if (url.protocol === "http:" && (scope !== "loopback" || !isLocalKind(kind))) {
    throw new AiGatewayError(
      "policy_rejected",
      "plaintext HTTP is allowed only for an explicitly local provider on loopback",
    );
  }
  return { url: url.toString(), origin: url.origin, scope };
}

export interface EndpointBinding {
  providerId: string;
  boundOrigin: string;
}

export function authorizeEndpointBinding(
  binding: EndpointBinding,
  endpoint: ValidatedEndpoint,
): void {
  if (binding.boundOrigin !== endpoint.origin) {
    throw new AiGatewayError(
      "policy_rejected",
      "provider endpoint origin changed; explicit secret re-binding is required",
    );
  }
}
