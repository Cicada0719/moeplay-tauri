import { invokeCmd } from "./core";
import type { SecretKind, SecretStatus } from "./types";

export async function secretStatus(
  kind: SecretKind,
  origin?: string | null,
): Promise<SecretStatus> {
  return invokeCmd("secret_status", { kind, origin: origin ?? null });
}

export async function secretSet(
  kind: SecretKind,
  secret: string,
  origin?: string | null,
): Promise<SecretStatus> {
  return invokeCmd("secret_set", { kind, origin: origin ?? null, secret });
}

export async function secretDelete(
  kind: SecretKind,
  origin?: string | null,
): Promise<SecretStatus> {
  return invokeCmd("secret_delete", { kind, origin: origin ?? null });
}
