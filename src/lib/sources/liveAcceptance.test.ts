import type { IncomingMessage } from "node:http";
import { describe, expect, it } from "vitest";
import { searchBaozi } from "./baoziProvider";
import { searchDm5 } from "./dm5Provider";
import { searchMangaDex } from "./mangadexProvider";

const live = process.env.MOEPLAY_LIVE_TESTS === "1";

async function requestText(url: string, redirects = 0): Promise<string> {
  const { request } = await import(url.startsWith("https:") ? "node:https" : "node:http");
  return new Promise((resolve, reject) => {
    const req = request(url, {
      headers: {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/131.0 Safari/537.36 MoePlay/0.12.0",
        Accept: "application/json,text/html,application/xhtml+xml,*/*;q=0.8",
        Referer: new URL(url).origin + "/",
      },
      timeout: 20_000,
    }, (response: IncomingMessage) => {
      const status = response.statusCode ?? 0;
      const location = response.headers.location;
      if (status >= 300 && status < 400 && location && redirects < 5) {
        response.resume();
        requestText(new URL(location, url).toString(), redirects + 1).then(resolve, reject);
        return;
      }
      const chunks: Buffer[] = [];
      response.on("data", (chunk: Buffer) => chunks.push(Buffer.from(chunk)));
      response.on("end", () => {
        const body = Buffer.concat(chunks).toString("utf8");
        status >= 200 && status < 300
          ? resolve(body)
          : reject(new Error(`${status} ${response.statusMessage ?? ""}: ${body.slice(0, 500)}`));
      });
    });
    req.on("timeout", () => req.destroy(new Error("request timeout")));
    req.on("error", reject);
    req.end();
  });
}

async function fetchMangaDex(url: string, _init?: RequestInit): Promise<Pick<Response, "status" | "json" | "ok">> {
  try {
    const text = await requestText(url);
    return { ok: true, status: 200, json: async () => JSON.parse(text) };
  } catch (error) {
    const match = String(error).match(/(?:Error:\s*)?(\d{3})/);
    return { ok: false, status: match ? Number(match[1]) : 500, json: async () => ({}) };
  }
}

async function fetchText(url: string): Promise<string> {
  return requestText(url);
}

describe.runIf(live)("live comic source acceptance", () => {
  it("keeps at least two independent public sources usable", async () => {
    const checks = await Promise.allSettled([
      searchMangaDex(fetchMangaDex, "Frieren", 5),
      searchBaozi(fetchText, "葬送的芙莉莲"),
      searchDm5(fetchText, "海贼王", "dm5"),
      searchDm5(fetchText, "海贼王", "ikkk"),
    ]);
    const healthy = checks
      .map((result, index) => ({ result, source: ["MangaDex", "Baozi", "DM5", "1kkk"][index] }))
      .filter(({ result }) => result.status === "fulfilled" && result.value.length > 0);
    const diagnostics = checks.map((result, index) =>
      result.status === "fulfilled"
        ? `${["MangaDex", "Baozi", "DM5", "1kkk"][index]}:${result.value.length}`
        : `${["MangaDex", "Baozi", "DM5", "1kkk"][index]}:${String(result.reason)}`,
    );
    console.info("live comic sources", diagnostics.join(" | "));
    expect(healthy.length, diagnostics.join(" | ")).toBeGreaterThanOrEqual(2);
  }, 40_000);
});
