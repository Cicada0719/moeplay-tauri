import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const defaultVersion = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;
const tauriConfig = JSON.parse(fs.readFileSync(path.join(root, "src-tauri", "tauri.conf.json"), "utf8"));
const defaultPublicKey = tauriConfig?.plugins?.updater?.pubkey;
const defaultSearchRoots = [path.join(root, "src-tauri", "target", "release")];
const rootManifest = path.join(root, "latest.json");

function walk(directory, predicate, matches = []) {
  if (!fs.existsSync(directory)) return matches;
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(full, predicate, matches);
    else if (entry.isFile() && predicate(entry.name, full)) matches.push(full);
  }
  return matches;
}

function normalizeText(value) {
  return value.replace(/\r\n/g, "\n").trim();
}

function decodeMinisignText(signature, label) {
  const normalized = normalizeText(signature);
  if (normalized.startsWith("untrusted comment:")) return normalized;
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(normalized)) {
    throw new Error(`${label}: signature is neither minisign text nor its base64 encoding`);
  }
  const decoded = Buffer.from(normalized, "base64").toString("utf8");
  if (!decoded.startsWith("untrusted comment:")) {
    throw new Error(`${label}: base64 signature does not contain minisign text`);
  }
  return normalizeText(decoded);
}

function minisignPayloadLines(text) {
  return text.split("\n").filter((line) => line && !line.includes("comment:"));
}

function decodePacket(payload, label, minimumBytes) {
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(payload)) {
    throw new Error(`${label}: contains an invalid minisign payload`);
  }
  const decoded = Buffer.from(payload, "base64");
  if (decoded.length < minimumBytes || decoded.toString("base64").replace(/=+$/, "") !== payload.replace(/=+$/, "")) {
    throw new Error(`${label}: minisign payload is malformed or unexpectedly short`);
  }
  return decoded;
}

function publicKeyId(publicKey, label) {
  if (typeof publicKey !== "string" || !publicKey.trim()) throw new Error(`${label}: updater public key is missing`);
  const text = decodeMinisignText(publicKey, label);
  const payload = minisignPayloadLines(text)[0];
  if (!payload) throw new Error(`${label}: updater public key payload is missing`);
  const packet = decodePacket(payload, label, 42);
  if (packet.subarray(0, 2).toString("ascii") !== "Ed") {
    throw new Error(`${label}: updater public key algorithm is not Ed25519`);
  }
  return packet.subarray(2, 10).toString("hex");
}

function validateMinisignSignature(signature, label, expectedKeyId) {
  if (typeof signature !== "string" || !signature.trim()) {
    throw new Error(`${label}: signature is missing`);
  }

  const lines = decodeMinisignText(signature, label).split("\n");
  if (lines.length < 2 || !lines[0].startsWith("untrusted comment:")) {
    throw new Error(`${label}: signature is not a minisign signature`);
  }

  const payloads = minisignPayloadLines(lines.join("\n"));
  if (!payloads.length) throw new Error(`${label}: signature payload is missing`);
  const signaturePacket = decodePacket(payloads[0], label, 74);
  if (!["Ed", "ED"].includes(signaturePacket.subarray(0, 2).toString("ascii"))) {
    throw new Error(`${label}: signature algorithm is not Ed25519`);
  }
  const signatureKeyId = signaturePacket.subarray(2, 10).toString("hex");
  if (expectedKeyId && signatureKeyId !== expectedKeyId) {
    throw new Error(`${label}: signature key ID does not match the configured updater public key`);
  }
  for (const payload of payloads.slice(1)) decodePacket(payload, label, 64);
  return signatureKeyId;
}

function validateHttpsUrl(value, label) {
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new Error(`${label}: URL is invalid`);
  }
  if (url.protocol !== "https:") throw new Error(`${label}: URL must use HTTPS`);
  if (url.username || url.password) throw new Error(`${label}: URL must not contain credentials`);
  if (url.hash) throw new Error(`${label}: URL must not contain a fragment`);
  return url;
}

function findLocalArtifact(url, artifactsDir) {
  if (!artifactsDir) return null;
  const assetName = decodeURIComponent(path.posix.basename(url.pathname));
  if (!assetName) throw new Error(`${url}: updater URL does not name an artifact`);
  const matches = walk(artifactsDir, (name) => name === assetName);
  if (matches.length !== 1) {
    throw new Error(`${assetName}: expected exactly one local updater artifact under ${artifactsDir}, found ${matches.length}`);
  }
  return matches[0];
}

export function verifyUpdaterManifest(manifestPath, options = {}) {
  const expectedVersion = options.expectedVersion ?? defaultVersion;
  const expectedKeyId = publicKeyId(options.publicKey ?? defaultPublicKey, `${manifestPath}: public key`);
  const artifactsDir = options.artifactsDir ? path.resolve(options.artifactsDir) : null;
  let data;
  try {
    data = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  } catch (error) {
    throw new Error(`${manifestPath}: cannot parse latest.json (${error.message})`);
  }

  if (!data || typeof data !== "object" || Array.isArray(data)) {
    throw new Error(`${manifestPath}: manifest must be a JSON object`);
  }
  if (data.version !== expectedVersion) {
    throw new Error(`${manifestPath}: version=${data.version ?? "<missing>"}, expected=${expectedVersion}`);
  }
  if (data.pub_date !== undefined && Number.isNaN(Date.parse(data.pub_date))) {
    throw new Error(`${manifestPath}: pub_date is not a valid date`);
  }

  const platforms = data.platforms && typeof data.platforms === "object" && !Array.isArray(data.platforms)
    ? Object.entries(data.platforms)
    : [];
  if (!platforms.length) throw new Error(`${manifestPath}: platforms is empty`);

  const verifiedArtifacts = [];
  for (const [platform, artifact] of platforms) {
    const label = `${manifestPath}: ${platform}`;
    if (!/^[a-z0-9]+-[a-z0-9_]+(?:-[a-z0-9]+)?$/i.test(platform)) {
      throw new Error(`${label}: platform key is invalid`);
    }
    if (!artifact || typeof artifact !== "object" || Array.isArray(artifact)) {
      throw new Error(`${label}: platform entry must be an object`);
    }
    const url = validateHttpsUrl(artifact.url, label);
    validateMinisignSignature(artifact.signature, label, expectedKeyId);

    const localArtifact = findLocalArtifact(url, artifactsDir);
    if (localArtifact) {
      if (!path.basename(localArtifact).includes(expectedVersion)) {
        throw new Error(`${label}: updater artifact filename does not contain version ${expectedVersion}`);
      }
      const signaturePath = `${localArtifact}.sig`;
      if (!fs.existsSync(signaturePath)) throw new Error(`${label}: detached signature is missing: ${signaturePath}`);
      const detachedSignature = fs.readFileSync(signaturePath, "utf8");
      validateMinisignSignature(detachedSignature, signaturePath, expectedKeyId);
      if (normalizeText(detachedSignature) !== normalizeText(artifact.signature)) {
        throw new Error(`${label}: latest.json signature does not match ${signaturePath}`);
      }
      verifiedArtifacts.push(localArtifact);
    }
  }

  return {
    manifestPath: path.resolve(manifestPath),
    platformCount: platforms.length,
    verifiedArtifacts,
    version: data.version,
  };
}

function parseArgs(argv) {
  const options = { require: false, expectAbsent: false, manifests: [], artifactsDir: null, expectedVersion: defaultVersion };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--require") options.require = true;
    else if (arg === "--expect-absent") options.expectAbsent = true;
    else if (arg === "--artifacts-dir") options.artifactsDir = path.resolve(argv[++index]);
    else if (arg === "--expected-version") options.expectedVersion = argv[++index];
    else if (arg.startsWith("-")) throw new Error(`Unknown option: ${arg}`);
    else options.manifests.push(path.resolve(arg));
  }
  if (options.require && options.expectAbsent) throw new Error("--require and --expect-absent cannot be combined");
  return options;
}

export function runCli(argv = process.argv.slice(2), runtime = {}) {
  const options = parseArgs(argv);
  let candidates = options.manifests;
  if (!candidates.length) {
    const searchRoots = runtime.searchRoots ?? defaultSearchRoots;
    const discovered = searchRoots.flatMap((directory) => walk(directory, (name) => name === "latest.json"));
    if (runtime.searchRoots === undefined && fs.existsSync(rootManifest)) discovered.unshift(rootManifest);
    candidates = [...new Set(discovered)];
  }

  if (!candidates.length) {
    const message = "Updater artifacts were not generated: latest.json is absent.";
    if (options.require) throw new Error(`${message} Signed release mode requires updater artifacts.`);
    console.log(`${message} This is expected for ordinary development and unsigned/degraded builds.`);
    return { status: "not-generated", manifests: [] };
  }
  if (options.expectAbsent) {
    throw new Error(`Updater artifacts must be absent in unsigned/degraded mode, but found: ${candidates.join(", ")}`);
  }

  const results = candidates.map((file) => verifyUpdaterManifest(file, options));
  for (const result of results) {
    const localCheck = options.artifactsDir
      ? `; ${result.verifiedArtifacts.length} detached signature(s) matched local artifacts`
      : "";
    console.log(`Updater manifest OK: ${path.relative(root, result.manifestPath)} (${result.platformCount} platform entries${localCheck})`);
  }
  return { status: "verified", manifests: results };
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;
if (isMain) {
  try {
    runCli();
  } catch (error) {
    console.error(`Updater verification failed: ${error.message}`);
    process.exitCode = 1;
  }
}
