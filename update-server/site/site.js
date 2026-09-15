const repo = "https://github.com/sgyxyx-prog/moeplay-tauri";
const el = id => document.getElementById(id);
const size = bytes => `${(bytes / 1024 / 1024).toFixed(1)} MB`;
const labels = { installer: "Windows · EXE 安装版", msi: "Windows · MSI", portable: "Windows · Portable ZIP", release: "Android · 正式 Release", compat: "Android · 旧签名兼容包" };
const assetUrl = (version, file) => `/downloads/${encodeURIComponent(version)}/${encodeURIComponent(file)}`;
function hasBoundAndroidCoverage(release, assets, channel) {
  const evidence = release.androidVerification?.[channel] ?? release.androidUpgradeReports?.[channel] ?? release.androidReports?.[channel];
  const report = evidence?.report ?? evidence;
  const asset = assets.find(item => item.channel === channel && item.platform === "android");
  const apkSha256 = report?.apkSha256 ?? report?.apkSHA256 ?? evidence?.apkSha256;
  const certificate = report?.certificateFingerprint ?? report?.certificateSha256 ?? report?.certFingerprint;
  const packageName = report?.packageName ?? report?.package;
  const device = report?.device ?? report?.deviceModel ?? evidence?.device;
  const checks = report?.dataCheck ?? report?.dataChecks;
  const checksPassed = checks && typeof checks === "object" && Object.keys(checks).length > 0 && Object.values(checks).every(check => check === true || check?.passed === true);
  return release.androidCompatibilityVerified === true && asset && typeof packageName === "string" && Number.isInteger(report?.versionCode ?? report?.toVersionCode)
    && /^[a-f0-9]{64}$/i.test(apkSha256 ?? "") && asset.sha256.toLowerCase() === apkSha256.toLowerCase()
    && /^(?:[a-f0-9]{2}:){15}[a-f0-9]{2}$|^[a-f0-9]{64}$/i.test(certificate ?? "")
    && (typeof device === "string" || (device && typeof device === "object"))
    && (report?.coverageUpgrade === true || report?.coveragePassed === true)
    && (report?.dataPreserved === true || report?.dataCheck === true || checksPassed);
}
async function loadRelease() {
  try {
    const response = await fetch("release-manifest.json", { cache: "no-store" });
    if (!response.ok) throw new Error("版本清单暂时不可用");
    const release = await response.json();
    if (!/^\d+\.\d+\.\d+$/.test(release.version) || !Array.isArray(release.assets)) throw new Error("版本清单格式错误");
    const assets = release.assets.filter(a => typeof a.file === "string" && !/[\\/]/.test(a.file) && Number.isFinite(a.size) && /^[a-f0-9]{64}$/.test(a.sha256));
    el("version-label").textContent = `v${release.version}`;
    el("release-status").textContent = `v${release.version} · ${new Date(release.publishedAt).toLocaleDateString("zh-CN")} 发布`;
    el("build-identity").textContent = `构建提交 ${release.commit} · GitHub 与官网下载使用同批产物`;
    for (const [id, channel] of [["windows-download", "installer"], ["android-download", "release"]]) {
      const asset = assets.find(a => a.channel === channel); if (!asset) continue;
      el(id).href = assetUrl(release.version, asset.file);
      el(id).querySelector("small").textContent = `v${release.version} · ${size(asset.size)} · ${asset.architecture.toUpperCase()}`;
    }
    for (const note of release.notes ?? []) { const li = document.createElement("li"); li.textContent = note; el("release-notes").append(li); }
    for (const asset of assets) {
      const row = document.createElement("article"); row.className = "asset";
      const copy = document.createElement("div"); const title = document.createElement("h3"); title.textContent = labels[asset.channel] ?? asset.file;
      const info = document.createElement("p"); info.textContent = `${asset.file} · ${size(asset.size)}`;
      const hash = document.createElement("p"); const code = document.createElement("code"); code.textContent = `SHA-256 ${asset.sha256}`; hash.append(code); copy.append(title, info, hash);
      const download = document.createElement("a"); download.href = assetUrl(release.version, asset.file); download.textContent = "下载 ↓"; download.setAttribute("aria-label", `下载 ${title.textContent}`);
      const alternate = document.createElement("a"); alternate.className = "alternate"; alternate.href = `${repo}/releases/download/v${release.version}/${encodeURIComponent(asset.file)}`; alternate.textContent = "GitHub 备用 ↗";
      row.append(copy, download, alternate); el("asset-list").append(row);
    }
    el("compatibility").textContent = hasBoundAndroidCoverage(release, assets, "release") && hasBoundAndroidCoverage(release, assets, "compat")
      ? "本版兼容包已完成旧证书核验和实机覆盖安装测试，可用于旧 Debug 渠道保留数据升级。仍建议先备份。"
      : "兼容包尚未完成实机覆盖验收，不承诺保留数据升级。请先备份，再选择与你当前安装一致的签名渠道。";
  } catch (error) { el("release-status").textContent = `${error.message}，请使用 GitHub 备用下载。`; }
}
async function loadVersions() {
  try {
    const response = await fetch("/versions.json", { cache: "no-store" }); if (!response.ok) return;
    const versions = await response.json();
    for (const version of versions) { if (!/^\d+\.\d+\.\d+$/.test(version)) continue; const link = document.createElement("a"); link.href = `/versions/${version}/`; link.textContent = `v${version} ↗`; el("version-list").prepend(link); }
  } catch { /* The GitHub archive remains available without server history metadata. */ }
}
void loadRelease(); void loadVersions();
