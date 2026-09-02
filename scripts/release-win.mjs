// 一条命令出内网自动更新版：tauri build（NSIS，自动 minisign 签名）→ 发布到局域网更新服务器。
// 用法：node scripts/release-win.mjs [--notes "更新说明"] [--skip-publish]
import path from "node:path";
import fs from "node:fs";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { loadConfig, publish } from "./publish-update.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function main() {
  const argv = process.argv.slice(2);
  let notes = null;
  let skipPublish = false;
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === "--notes") notes = argv[++i];
    else if (argv[i] === "--skip-publish") skipPublish = true;
    else throw new Error(`未知参数：${argv[i]}`);
  }
  const cfg = loadConfig();
  const version = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;
  const cli = path.join(root, "node_modules", "@tauri-apps", "cli", "tauri.js");
  console.log(`▶ tauri build --bundles nsis（v${version}，产物将自动用配置中的私钥签名）`);
  const res = spawnSync(process.execPath, [cli, "build", "--bundles", "nsis"], {
    stdio: "inherit",
    env: {
      ...process.env,
      TAURI_SIGNING_PRIVATE_KEY: fs.readFileSync(cfg.privateKeyPath, "utf8"),
      TAURI_SIGNING_PRIVATE_KEY_PASSWORD: cfg.password,
    },
  });
  if (res.status !== 0) {
    console.error("✖ 构建失败");
    process.exitCode = res.status ?? 1;
    return;
  }
  if (skipPublish) {
    console.log("✔ 构建完成（已按 --skip-publish 跳过发布）");
    return;
  }
  return publish({ dir: null, version, notes, configPath: undefined, dryRun: false }).catch((error) => {
    console.error(`发布失败：${error.message}`);
    process.exitCode = 1;
  });
}

main();
