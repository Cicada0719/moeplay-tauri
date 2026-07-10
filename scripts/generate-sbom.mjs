import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const pkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const lock = JSON.parse(fs.readFileSync(path.join(root, "package-lock.json"), "utf8"));
const cargoLock = fs.readFileSync(path.join(root, "src-tauri", "Cargo.lock"), "utf8");
const outputDir = path.join(root, "src-tauri", "target", "release", "bundle");
const output = path.join(outputDir, "sbom.cdx.json");

function cargoPackages(text) {
  return [...text.matchAll(/\[\[package\]\]\s+name = "([^"]+)"\s+version = "([^"]+)"(?:\s+source = "([^"]+)")?/g)]
    .map(([, name, version, source]) => ({ type: "library", name, version, scope: "required", purl: `pkg:cargo/${encodeURIComponent(name)}@${encodeURIComponent(version)}`, properties: source ? [{ name: "cargo:source", value: source }] : undefined }));
}

function npmPackages(packages) {
  const components = [];
  for (const [location, value] of Object.entries(packages ?? {})) {
    if (!location || !value?.version) continue;
    const marker = "node_modules/";
    const index = location.lastIndexOf(marker);
    if (index < 0) continue;
    const name = location.slice(index + marker.length);
    if (!name) continue;
    components.push({ type: "library", name, version: value.version, scope: value.dev ? "optional" : "required", purl: `pkg:npm/${encodeURIComponent(name)}@${encodeURIComponent(value.version)}` });
  }
  return components;
}

const byPurl = new Map();
for (const component of [...npmPackages(lock.packages), ...cargoPackages(cargoLock)]) byPurl.set(component.purl, component);
const components = [...byPurl.values()].sort((a, b) => a.purl.localeCompare(b.purl));
const serialSeed = JSON.stringify({ version: pkg.version, components: components.map(({ purl }) => purl) });
const serial = crypto.createHash("sha256").update(serialSeed).digest("hex").slice(0, 32);
const document = {
  bomFormat: "CycloneDX",
  specVersion: "1.5",
  serialNumber: `urn:uuid:${serial.slice(0, 8)}-${serial.slice(8, 12)}-${serial.slice(12, 16)}-${serial.slice(16, 20)}-${serial.slice(20)}`,
  version: 1,
  metadata: {
    timestamp: new Date().toISOString(),
    tools: [{ vendor: "MoePlay", name: "generate-sbom.mjs", version: "1" }],
    component: { type: "application", name: pkg.productName ?? pkg.name, version: pkg.version, purl: `pkg:generic/moeplay@${encodeURIComponent(pkg.version)}` },
  },
  components,
};
fs.mkdirSync(outputDir, { recursive: true });
fs.writeFileSync(output, `${JSON.stringify(document, null, 2)}\n`);
console.log(output);
console.log(`SBOM components: ${components.length}`);
