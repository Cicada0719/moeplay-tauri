#!/usr/bin/env node

import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const DEFAULT_ROOT = path.resolve(SCRIPT_DIR, "..");
const FRONTEND_EXTENSIONS = new Set([".js", ".jsx", ".mjs", ".ts", ".tsx", ".svelte"]);

export function kebabToSnake(value) {
  return value.replaceAll("-", "_");
}

export function snakeToKebab(value) {
  return value.replaceAll("_", "-");
}

function stripComments(source) {
  let result = "";
  let index = 0;
  let state = "code";
  let blockDepth = 0;

  while (index < source.length) {
    const char = source[index];
    const next = source[index + 1];

    if (state === "line-comment") {
      if (char === "\n") {
        state = "code";
        result += char;
      } else {
        result += " ";
      }
      index += 1;
      continue;
    }

    if (state === "block-comment") {
      if (char === "/" && next === "*") {
        blockDepth += 1;
        result += "  ";
        index += 2;
      } else if (char === "*" && next === "/") {
        blockDepth -= 1;
        result += "  ";
        index += 2;
        if (blockDepth === 0) state = "code";
      } else {
        result += char === "\n" ? "\n" : " ";
        index += 1;
      }
      continue;
    }

    if (state === "string") {
      result += char;
      if (char === "\\") {
        if (next !== undefined) result += next;
        index += 2;
      } else {
        index += 1;
        if (char === '"') state = "code";
      }
      continue;
    }

    if (state === "char") {
      result += char;
      if (char === "\\") {
        if (next !== undefined) result += next;
        index += 2;
      } else {
        index += 1;
        if (char === "'") state = "code";
      }
      continue;
    }

    if (char === "/" && next === "/") {
      state = "line-comment";
      result += "  ";
      index += 2;
    } else if (char === "/" && next === "*") {
      state = "block-comment";
      blockDepth = 1;
      result += "  ";
      index += 2;
    } else if (char === '"') {
      state = "string";
      result += char;
      index += 1;
    } else if (char === "'") {
      state = "char";
      result += char;
      index += 1;
    } else {
      result += char;
      index += 1;
    }
  }

  return result;
}

function extractBalanced(source, openIndex, openChar, closeChar, context) {
  let depth = 0;
  let quote = null;

  for (let index = openIndex; index < source.length; index += 1) {
    const char = source[index];

    if (quote !== null) {
      if (char === "\\") {
        index += 1;
      } else if (char === quote) {
        quote = null;
      }
      continue;
    }

    if (char === '"' || char === "'") {
      quote = char;
    } else if (char === openChar) {
      depth += 1;
    } else if (char === closeChar) {
      depth -= 1;
      if (depth === 0) return source.slice(openIndex + 1, index);
    }
  }

  throw new Error(`Unclosed ${openChar}${closeChar} block while parsing ${context}`);
}

function duplicates(values) {
  const seen = new Set();
  const repeated = new Set();
  for (const value of values) {
    if (seen.has(value)) repeated.add(value);
    seen.add(value);
  }
  return [...repeated].sort();
}

export function parseGenerateHandler(source) {
  const clean = stripComments(source);
  const markerIndex = clean.indexOf("generate_handler!");
  if (markerIndex < 0) throw new Error("Could not find generate_handler! in src-tauri/src/lib.rs");

  const openIndex = clean.indexOf("[", markerIndex);
  if (openIndex < 0) throw new Error("Could not find generate_handler![...] command list");

  const body = extractBalanced(clean, openIndex, "[", "]", "generate_handler!");
  const commands = [...body.matchAll(/\b(?:[A-Za-z_][A-Za-z0-9_]*::)+([A-Za-z_][A-Za-z0-9_]*)\s*,/g)].map(
    (match) => match[1]
  );
  if (commands.length === 0) throw new Error("generate_handler! did not contain any command paths");
  return { commands, duplicates: duplicates(commands) };
}

export function parseBuildCommands(source) {
  const clean = stripComments(source);
  const markerIndex = clean.search(/\bconst\s+COMMANDS\b/);
  if (markerIndex < 0) throw new Error("Could not find const COMMANDS in src-tauri/build.rs");

  const assignmentIndex = clean.indexOf("=", markerIndex);
  const openIndex = clean.indexOf("[", assignmentIndex);
  if (assignmentIndex < 0 || openIndex < 0) throw new Error("Could not find COMMANDS = &[...] command list");

  const body = extractBalanced(clean, openIndex, "[", "]", "build.rs COMMANDS");
  const commands = [...body.matchAll(/"([A-Za-z0-9_-]+)"/g)].map((match) => match[1]);
  if (commands.length === 0) throw new Error("build.rs COMMANDS did not contain any commands");
  return { commands, duplicates: duplicates(commands) };
}

function isFrontendSource(filePath) {
  const basename = path.basename(filePath);
  if (!FRONTEND_EXTENSIONS.has(path.extname(filePath))) return false;
  if (basename.endsWith(".d.ts")) return false;
  return !/(?:^|\.)(?:test|spec)\.[^.]+$/.test(basename);
}

async function walkFrontendFiles(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === "__tests__" || entry.name.startsWith(".")) continue;
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await walkFrontendFiles(entryPath)));
    else if (entry.isFile() && isFrontendSource(entryPath)) files.push(entryPath);
  }
  return files;
}

function skipTrivia(source, start) {
  let index = start;
  while (index < source.length) {
    if (/\s/.test(source[index])) {
      index += 1;
    } else if (source[index] === "/" && source[index + 1] === "/") {
      index = source.indexOf("\n", index + 2);
      if (index < 0) return source.length;
    } else if (source[index] === "/" && source[index + 1] === "*") {
      const end = source.indexOf("*/", index + 2);
      return end < 0 ? source.length : skipTrivia(source, end + 2);
    } else {
      break;
    }
  }
  return index;
}

function skipQuoted(source, start) {
  const quote = source[start];
  let hasInterpolation = false;
  let value = "";

  for (let index = start + 1; index < source.length; index += 1) {
    const char = source[index];
    if (char === "\\") {
      if (index + 1 < source.length) {
        value += source[index + 1];
        index += 1;
      }
    } else if (quote === "`" && char === "$" && source[index + 1] === "{") {
      hasInterpolation = true;
      value += "${";
      index += 1;
    } else if (char === quote) {
      return { end: index + 1, value, hasInterpolation };
    } else {
      value += char;
    }
  }

  return { end: source.length, value, hasInterpolation: true };
}

function skipTypeArguments(source, start) {
  let depth = 0;
  let index = start;
  while (index < source.length) {
    const char = source[index];
    if (char === '"' || char === "'" || char === "`") {
      index = skipQuoted(source, index).end;
      continue;
    }
    if (char === "<") depth += 1;
    else if (char === ">") {
      depth -= 1;
      if (depth === 0) return index + 1;
    }
    index += 1;
  }
  return source.length;
}

function lineNumberAt(source, index) {
  let line = 1;
  for (let cursor = 0; cursor < index; cursor += 1) {
    if (source[cursor] === "\n") line += 1;
  }
  return line;
}

export function parseFrontendInvokeCommands(source, fileLabel = "<frontend>") {
  const commands = [];
  const dynamicCalls = [];
  let index = 0;
  let previousIdentifier = null;

  while (index < source.length) {
    const char = source[index];
    const next = source[index + 1];

    if (char === "/" && next === "/") {
      const lineEnd = source.indexOf("\n", index + 2);
      index = lineEnd < 0 ? source.length : lineEnd + 1;
      continue;
    }
    if (char === "/" && next === "*") {
      const commentEnd = source.indexOf("*/", index + 2);
      index = commentEnd < 0 ? source.length : commentEnd + 2;
      continue;
    }
    if (char === '"' || char === "'" || char === "`") {
      index = skipQuoted(source, index).end;
      continue;
    }
    if (!/[A-Za-z_$]/.test(char)) {
      index += 1;
      continue;
    }

    const identifierStart = index;
    index += 1;
    while (index < source.length && /[A-Za-z0-9_$]/.test(source[index])) index += 1;
    const identifier = source.slice(identifierStart, index);

    if (identifier !== "invokeCmd" || previousIdentifier === "function") {
      previousIdentifier = identifier;
      continue;
    }

    let cursor = skipTrivia(source, index);
    if (source[cursor] === "<") cursor = skipTrivia(source, skipTypeArguments(source, cursor));
    if (source[cursor] !== "(") {
      previousIdentifier = identifier;
      continue;
    }

    cursor = skipTrivia(source, cursor + 1);
    const quote = source[cursor];
    if (quote !== '"' && quote !== "'" && quote !== "`") {
      dynamicCalls.push(`${fileLabel}:${lineNumberAt(source, identifierStart)}`);
      previousIdentifier = identifier;
      continue;
    }

    const literal = skipQuoted(source, cursor);
    if (literal.hasInterpolation || !/^[A-Za-z0-9_-]+$/.test(literal.value)) {
      dynamicCalls.push(`${fileLabel}:${lineNumberAt(source, identifierStart)}`);
    } else {
      commands.push(literal.value);
    }
    previousIdentifier = identifier;
  }

  return { commands, dynamicCalls };
}

async function readCapabilities(capabilitiesDirectory) {
  const commands = new Set();
  const files = (await readdir(capabilitiesDirectory, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith(".json"))
    .map((entry) => entry.name)
    .sort();

  if (files.length === 0) throw new Error("No capability JSON files found");

  for (const file of files) {
    const filePath = path.join(capabilitiesDirectory, file);
    const capability = JSON.parse(await readFile(filePath, "utf8"));
    if (!Array.isArray(capability.permissions)) continue;
    for (const permission of capability.permissions) {
      if (typeof permission === "string" && /^allow-[A-Za-z0-9-]+$/.test(permission)) {
        commands.add(kebabToSnake(permission.slice("allow-".length)));
      }
    }
  }

  return { commands, files };
}


async function readGeneratedPermissions(directory) {
  const files = (await readdir(directory, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith(".toml"))
    .map((entry) => entry.name)
    .sort();
  if (files.length === 0) throw new Error("No autogenerated command permission TOML files found");
  return { commands: new Set(files.map((file) => file.slice(0, -".toml".length))), files };
}

function setDifference(left, right) {
  return [...left].filter((value) => !right.has(value)).sort();
}

function difference(label, left, right) {
  const values = setDifference(left, right);
  return values.length === 0 ? null : { label, values };
}

export async function inspectCommandContract(projectRoot) {
  const root = path.resolve(projectRoot);
  const libPath = path.join(root, "src-tauri", "src", "lib.rs");
  const buildPath = path.join(root, "src-tauri", "build.rs");
  const capabilitiesPath = path.join(root, "src-tauri", "capabilities");
  const frontendPath = path.join(root, "src");
  const generatedPermissionsPath = path.join(root, "src-tauri", "permissions", "autogenerated");

  const [handlerSource, buildSource, capabilities, generatedPermissions, frontendFiles] = await Promise.all([
    readFile(libPath, "utf8"),
    readFile(buildPath, "utf8"),
    readCapabilities(capabilitiesPath),
    readGeneratedPermissions(generatedPermissionsPath),
    walkFrontendFiles(frontendPath),
  ]);

  const handler = parseGenerateHandler(handlerSource);
  const build = parseBuildCommands(buildSource);
  const frontendCommands = new Set();
  const dynamicCalls = [];

  for (const filePath of frontendFiles.sort()) {
    const relativePath = path.relative(root, filePath).replaceAll(path.sep, "/");
    const parsed = parseFrontendInvokeCommands(await readFile(filePath, "utf8"), relativePath);
    for (const command of parsed.commands) frontendCommands.add(command);
    dynamicCalls.push(...parsed.dynamicCalls);
  }

  const handlerCommands = new Set(handler.commands);
  const buildCommands = new Set(build.commands);
  const permissionCommands = capabilities.commands;
  const differences = [
    difference("Missing from build.rs COMMANDS (generate_handler! only)", handlerCommands, buildCommands),
    difference("Extra in build.rs COMMANDS (not in generate_handler!)", buildCommands, handlerCommands),
    difference("Missing custom capability allow permissions", handlerCommands, permissionCommands),
    difference("Missing autogenerated command permission files", handlerCommands, generatedPermissions.commands),
    difference("Extra autogenerated command permission files", generatedPermissions.commands, handlerCommands),
    difference("Extra custom capability allow permissions", permissionCommands, handlerCommands),
    difference("Frontend invokeCmd commands not registered", frontendCommands, handlerCommands),
  ].filter(Boolean);

  const duplicateProblems = [];
  if (handler.duplicates.length > 0) {
    duplicateProblems.push({ label: "Duplicate generate_handler! commands", values: handler.duplicates });
  }
  if (build.duplicates.length > 0) {
    duplicateProblems.push({ label: "Duplicate build.rs COMMANDS entries", values: build.duplicates });
  }
  if (dynamicCalls.length > 0) {
    duplicateProblems.push({ label: "Non-literal frontend invokeCmd calls (cannot verify)", values: dynamicCalls.sort() });
  }

  return {
    root,
    counts: {
      generateHandler: handlerCommands.size,
      buildCommands: buildCommands.size,
      capabilityCommands: permissionCommands.size,
      generatedPermissionCommands: generatedPermissions.commands.size,
      frontendCommands: frontendCommands.size,
    },
    capabilityFiles: capabilities.files,
    generatedPermissionFiles: generatedPermissions.files,
    frontendFileCount: frontendFiles.length,
    differences,
    problems: duplicateProblems,
    ok: differences.length === 0 && duplicateProblems.length === 0,
  };
}

function formatGroup(group) {
  return [`- ${group.label} (${group.values.length}):`, ...group.values.map((value) => `    ${value}`)].join("\n");
}

export function formatReport(report) {
  const lines = [
    `[command-contract] ${report.ok ? "OK" : "FAILED"}`,
    `  generate_handler!: ${report.counts.generateHandler}`,
    `  build.rs COMMANDS: ${report.counts.buildCommands}`,
    `  capability custom allows: ${report.counts.capabilityCommands} (${report.capabilityFiles.join(", ")})`,
    `  autogenerated permissions: ${report.counts.generatedPermissionCommands}`,
    `  frontend invokeCmd commands: ${report.counts.frontendCommands} (${report.frontendFileCount} files scanned)`,
  ];

  const groups = [...report.differences, ...report.problems];
  if (groups.length > 0) lines.push("", ...groups.map(formatGroup));
  return lines.join("\n");
}

function parseArguments(argv) {
  let root = DEFAULT_ROOT;
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--root") {
      if (!argv[index + 1]) throw new Error("--root requires a directory argument");
      root = path.resolve(argv[index + 1]);
      index += 1;
    } else if (argument === "--help" || argument === "-h") {
      return { help: true, root };
    } else {
      throw new Error(`Unknown argument: ${argument}`);
    }
  }
  return { help: false, root };
}

async function main() {
  const options = parseArguments(process.argv.slice(2));
  if (options.help) {
    console.log("Usage: node scripts/verify-command-contract.mjs [--root <project-root>]");
    return;
  }

  const report = await inspectCommandContract(options.root);
  console.log(formatReport(report));
  if (!report.ok) process.exitCode = 1;
}

const isMain = process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url;
if (isMain) {
  main().catch((error) => {
    console.error(`[command-contract] ERROR: ${error.message}`);
    process.exitCode = 1;
  });
}
