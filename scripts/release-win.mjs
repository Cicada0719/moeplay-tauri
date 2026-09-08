// 本地构建官方 Windows 安装包（NSIS、MSI，自动更新签名）。
// 用法：node scripts/release-win.mjs [--notes "更新说明"] [--skip-publish]
import path from "node:path";
import fs from "node:fs";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { loadSigningConfig } from "./publish-update.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function main() {
  const argv = process.argv.slice(2);
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === "--notes") ++i; // Accepted for callers of the old build-and-upload command.
    else if (argv[i] === "--skip-publish") continue;
    else throw new Error(`未知参数：${argv[i]}`);
  }
  const cfg = loadSigningConfig();
  const version = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;
  const cli = path.join(root, "node_modules", "@tauri-apps", "cli", "tauri.js");
  console.log(`▶ tauri build --bundles nsis,msi（v${version}，产物将自动用配置中的私钥签名）`);
  const res = spawnSync(process.execPath, [cli, "build", "--bundles", "nsis,msi", "--config", "src-tauri/tauri.official.conf.json"], {
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
  console.log("✔ 本地构建完成。完成安装验收后，将同批文件发布到 GitHub 并通过 SFTP 上传下载站。");
}

main();
