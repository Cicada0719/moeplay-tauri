# 萌游 MoeGame · Codex 施工方案 R3 —— 验收修复 + Steam 无 Key 离线导入

> 版本：R3 · 生成：2026-06-12 · 指挥/策划：Claude（真机验收 + 日志定位）· 执行：Codex
> 本文承接 [`CODEX_PLAN_R2.md`](./CODEX_PLAN_R2.md)（设计总纲 / Aura 皮肤 / G2·G4·E·H 任务卡仍然有效，**不重复**）。
> R3 只解决两件事：① 记录本轮真机验收发现 + 已落地的修复（§1）；② 给出**新的优先级与任务卡**（§2 起），头号是 **P0：Steam 登录导入在「无 API Key + 网络被墙」下也能真正灌进游戏**。

---

## 0. 一句话现状

主界面、灌库（520 游戏）、双壳（Switch/PS5）都在。本轮真机验收发现**两个会被用户一眼看到的硬伤**——封面全裂、Steam 网页登录拿不到游戏——**已由 Claude 当场修掉并真机确认**（§1）。R3 的重心转向：把 Steam 导入做成**像 Playnite 一样、不需要 API Key、断网也能用**（§2 · P0）。

---

## 1. 本轮验收结论与已落地修复（Claude · 2026-06-12 真机）

| # | 问题 | 根因 | 状态 | 证据/改动 |
|---|---|---|---|---|
| **F1** | **全库封面裂图**（磁贴只剩游戏名 alt 文本 + 碎图标） | **资源协议作用域写错**：Rust 实际把封面写在 `dirs::data_dir()/moeplay`（=`%APPDATA%\moeplay`），但 `tauri.conf.json` 的 `assetProtocol.scope` 用了 `$APPDATA/moeplay/**`——Tauri 2 里 `$APPDATA` 解析为 **app 专属目录** `%APPDATA%\com.moeplay.app`（不存在），且 `$LOCALAPPDATA` 根本不是合法变量。所有 `asset://` 请求都在作用域外被拒。 | ✅ **已修·真机确认** | `tauri.conf.json` scope 改为 `$DATA/moeplay/**` + `$LOCALDATA/moeplay/**`（再加 `$APPDATA`/`$APPLOCALDATA` 前向兼容）。重启后封面全部正常渲染（Dead by Daylight / P5R / Apex 等截图为证）。 |
| **F2** | Steam 网页登录：**默认已登录态下，点开再关掉拿不到游戏** | 登录窗起始 URL 是 `/login/home`；已登录时页面仍停在含 `login` 的地址，而轮询的「跳转到个人页」逻辑显式 `!contains("login")` 跳过它 → 永远解析不出 SteamID。 | ✅ **已修·日志确认** | `steam_openid.rs`：起始 URL 改 `/my/profile`（已登录直接 302 到 `/profiles/{id}`；未登录则跳带二维码的登录页，登录后回跳）。新增 `g_steamID` 探针注入，兼容 `/id/{vanity}` 个性链接。日志已打印 `Steam login detected via navigation, sid: 765611992206...`。 |
| **F3** | 登录成功后**登录窗不自动关闭**、停在个人页 | `on_navigation` 检测到并发了事件，但只有轮询循环里 `wc.url()` 命中 `/profiles/` 才关窗；URL 停在 vanity 页时轮询再也命中不到。 | ✅ **已修** | `steam_openid.rs`：轮询循环顶部加 `if emitted { sleep(1s); wc.close() }` 兜底关窗（无论哪条路径检测到都关）。 |
| **F4** | 每次启动**清空整盘缩略图缓存** → 500+ 封面每次重生成、首屏慢 | `lib.rs` setup 注释写「>30 天清理」，实际调用的是 `clear_thumbnail_cache()`（全清）。 | ✅ **已修** | 新增 `thumbnail::prune_thumbnails(30)`（只删 30 天未更新的），`lib.rs` 改调它。启动日志不再出现 "Thumbnail cache cleared"。 |
| **F5** | 登录窗被用户手动关闭后，前端卡在「等待登录」 | 后端发 `status:"closed"`，前端 `moe://steam-progress` 只处理 `timeout`。 | ✅ **已修** | `PlatformImportPage.svelte`：`closed`/`timeout` 都复位 `openingLogin`。 |

**回归验证**：`cargo test --lib` 94 passed / 0 failed；`svelte-check` 0 error / 0 warning；真机封面正常、SteamID 解析正常。

> ⚠️ **F1 给 Codex 的硬规矩**：本项目所有用户数据落在 `dirs::data_dir()/moeplay`（漫游）与 `dirs::cache_dir()/moeplay`（本地）。**任何新增需要被 WebView 读取的媒体目录，资源作用域必须用 `$DATA` / `$LOCALDATA`（系统级），不要用 `$APPDATA`/`$APPLOCALDATA`（app 专属，指向 `com.moeplay.app`，与实际落盘不一致）。**

---

## 2. P0 ⭐⭐⭐ · Steam「无 Key · 离线优先」全库导入（对标 Playnite）

### 2.1 为什么这是 P0
用户原话：「网页登录扫码 steam，打开是默认登录态，点退出来还是没正常获取到游戏」。本轮定位到**真正的缺口**：
- **登录只解析身份**，拉全库 `GetOwnedGames` **必须 Steam Web API Key**——用户没填，自然 0 游戏。
- 用户网络**封了 Steam CDN/OpenID（Akamai Access Denied）**，连 `api.steampowered.com` 也可能不通。靠云端 API 这条路在该网络下天然脆弱。
- 但**本机 Steam 客户端的数据是齐的**（真机已确认）：
  - `Steam/userdata/1260412624/config/localconfig.vdf`（58KB）→ **拥有游戏 + 时长 + 最后游玩**，离线可读。`1260412624` 即 SteamID64 低 32 位（`76561197960265728 + accountid = 76561199220678352`，与检测到的一致）。
  - `Steam/appcache/librarycache/{appid}/library_600x900.jpg`（1047 个 appid 缓存）→ **竖版封面**，外加 `library_hero.jpg`（背景）、`logo.png`。离线可读。
  - `Steam/steamapps/*.acf` → 已安装（本机仅 4 个，所以只扫已安装远不够，用户拥有 98 个）。

**结论**：把 Steam 导入的主路径从「云端 API」改成「**本机 Steam 文件优先 + 网页会话兜底**」，无 Key、断网可用，且封面直接复用 Steam 已下好的缓存——最契合该用户，也最像 Playnite。

### 2.2 任务卡 S1 · 本机 Steam 全量解析（离线 · 无 Key）【主路径】
- **找账号**：读 `Steam/config/loginusers.vdf` 取 `MostRecent=1` 的 SteamID64 → 推出 accountid（`steamid64 & 0xFFFFFFFF`）→ 定位 `userdata/{accountid}/`。多账号时用当前已检测/登录的 SteamID 优先。
- **解析拥有游戏 + 时长**：解析 `userdata/{accountid}/config/localconfig.vdf`（VDF 文本格式）里 `Software/Valve/Steam/apps/{appid}` 节点 → `Playtime`(分钟,全部) / `Playtime2wks` / `LastPlayed`(unix)。这一份就是「这台机器登录过的账号拥有/玩过的库」。
  - 补充源（更全）：`appcache/appinfo.vdf`（二进制 VDF）含游戏名；拿不到名字的用 appid 兜底，名字可后续刮削补。
- **封面/背景（离线）**：`appcache/librarycache/{appid}/library_600x900.jpg` → 竖封面；`library_hero.jpg` → 背景；`logo.png` → 图标。导入时**复制进 `dirs::data_dir()/moeplay/covers|backgrounds|icons`**（沿用迁移那套 `copy_if_exists`，存绝对路径），这样作用域（F1 已修）能渲染、且不依赖网络。
- **映射**：复用 `PlatformGameCandidate` + `import_platform_library`（去重/合并/不覆盖用户手改的中文名与评分）。`launch_uri = steam://rungameid/{appid}`，`platform="Steam"`。
- **VDF 解析**：引入轻量 VDF parser（`keyvalues-serde` / `steamy-vdf`，或自己写一个文本 VDF 递归解析；二进制 appinfo 可选）。**先确认 `package.json`/`Cargo.toml`，缺则在卡里写明新增依赖**。
- **验收**：本机有 Steam、未填 Key、可断网 → 点一次「本机 Steam 全量导入」→ 库里出现该账号拥有的全部游戏（本例 ~98），带竖封面 + 时长 + 最后游玩；重入幂等；`cargo test` 覆盖 localconfig.vdf 解析（造一个小样例）。

### 2.3 任务卡 S2 · 网页会话兜底抓取（无 Key · 在线）【localconfig 拿不到时】
- 适用：本机没装 Steam，或 localconfig 不含某些只拥有未安装的游戏。
- 复用已修好的登录窗（`/my/profile` + g_steamID 探针）。登录成功拿到 SteamID 后，**同一 WebView 导航到** `https://steamcommunity.com/profiles/{steamid}/games/?tab=all`（自己的会话即使资料私密也可见）。
- **注入 JS 抽取**：页面渲染用的全局 `var rgGames = [...]`（JSON 数组：`appid / name / logo / playtime_forever(分钟) / playtime_2weeks / last_played`）。注入脚本 `JSON.stringify(rgGames)` 后**经自定义协议回传**：`location.href = 'moe-steam-games://ingest?d=' + encodeURIComponent(json)`；后端 `on_navigation` 截获 `moe-steam-games://` → 解码 → 解析 → 走 `import_platform_library` → 返回 `false` 取消导航。
  - 长度兜底：>~1MB（约 5000+ 游戏）时分片回传或退回 XML 端点 `/games?tab=all&xml=1` 解析。
- 封面：rgGames 给的是横版 `logo` hash；竖封面用 appid 拼 `library_600x900` URL，**但优先查本机 librarycache**（见 S1），CDN 被墙时不至于裂图。
- **验收**：无本机 Steam、未填 Key → 网页登录后自动抓取并导入全库（带时长）；断 CDN 时封面回退本机缓存或占位不报错。

### 2.4 任务卡 S3 · API Key 仍作为「增强档」（可选）
- 保留现有 `GetOwnedGames`/`GetPlayerAchievements` 路径作为**可选增强**：填了 Key 才拉**成就**（`achievements_total/unlocked`）与更准的元数据。
- UX：连接区把三条路径讲清楚——**①本机一键导入（推荐·无需 Key）｜②网页登录抓取（无本机 Steam 时）｜③填 API Key（额外同步成就）**。当前 UI 把 Key 摆在最显眼且「缺 API Key」红字会误导用户以为必填，需调整文案与主次。
- 注：Claude 已做的成就拉取优化（只对 `playtime_forever>0` 的游戏拉、加 10s 超时、并发 8）保留。

---

## 3. P1 · 封面韧性与既有 R2 收尾

### 3.1 卡 R1 · Steam/远程封面被墙回退
- 现状：部分已导入 Steam 游戏封面是 `cdn.*.steamstatic.com/.../library_600x900.jpg` 远程 URL（本例 2 个），CDN 被墙即裂图；另有 13 个游戏完全无封面。
- 改：`CachedImage`/`cache_thumbnail` 对远程图失败时——①试多 CDN 主机（akamai/cloudflare/fastly 轮替）；②回退本机 `librarycache/{appid}/library_600x900.jpg`；③**导入时就把远程封面落盘到本地 covers/**（不再运行期依赖网络）。无封面的游戏走自动刮削补图或显示更体面的占位（当前是首字母 mono，可接受）。
- 验收：断网/被墙下，已导入 Steam 游戏封面仍显示（来自本机缓存或落盘副本）。

### 3.2 沿用 R2 未完任务卡（**细节见 [`CODEX_PLAN_R2.md`](./CODEX_PLAN_R2.md)，此处只列状态**）
| 卡 | 内容 | 状态 | 备注 |
|---|---|---|---|
| **G2** | 全库网格 + 游戏详情 | 进行中 | `GameGrid`/`GameDetailPage` 已在，需按 R2 §5 验收无空白 |
| **G4** | 工具页 **Aura 绮境**皮肤（R2 §1.7，先 G4a 基建） | 未开始 | 平台导入页已隐约有 Aura 雏形（kicker+玫红 tick），可作 G4 起点 |
| **E** | 数据模型收敛（写入只走 `metadata.*`） | 部分 | `utils/game.ts` 取值器已兜底 |
| **H** | `commands.rs` 拆分收尾 | 部分 | 仍 **1359 行**；`commands/` 目录已建（platform/play/saves/games/metadata…），把剩余命令迁出，`cargo fmt --check && clippy -D warnings` 绿 |

---

## 4. P2 · 小修小补（顺手）
- **F4 延伸**：缩略图磁盘缓存键含源文件 mtime，封面被重新刮削（同名覆盖）后能自动失效，免得 30 天内显示旧图。
- 13 个无封面游戏：入库后台自动刮削补封面。
- 登录窗 `on_navigation` 已能关窗（F3），但 `wc.url()` 在部分 WebView2 版本返回创建时 URL——已用 `emitted` 兜底，无需再改；若后续发现轮询误判，改用 `on_page_load` 注入探针。

---

## 5. 关键路径与里程碑
1. **S1（本机离线导入）= P0 主攻**，解锁「该用户真正能把 Steam 库灌进来」——独立于网络与 Key，应**最先做**。
2. S2 网页兜底、R1 封面韧性紧随其后（覆盖无本机 Steam / 远程封面场景）。
3. G2/G4/E/H 按 R2 继续，不被 R3 阻塞。

| M | 目标 | 含卡 | 出口 |
|---|---|---|---|
| **R3-M1** | Steam 真能用 | **S1**（+S3 文案） | 无 Key、可断网，一键把本机 Steam 账号全库（~98）灌入，带竖封面+时长 |
| **R3-M2** | 全场景覆盖 | **S2 / R1** | 无本机 Steam 也能网页抓全库；被墙网络封面不裂 |
| **R3-M3** | 收口 | R2 的 **G2/G4/E/H** | 见 R2 §4 |

### Definition of Done（每卡）
1. `cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings && cargo test` 绿；前端 `npx svelte-check` 0 error。
2. **真机自验证据**：截图/录屏/日志贴 PR（Steam 卡必须展示「未填 Key」状态下成功导入）。
3. 禁裸 `unwrap/expect`（main/测试除外）；不把原始异常抛给用户。
4. 一卡一 PR；媒体目录作用域只用 `$DATA`/`$LOCALDATA`（见 §1 F1 硬规矩）。

## 6. 一句话给 Codex
封面已修好（资源作用域 `$APPDATA→$DATA`）、Steam 登录已能解析身份。**你的头号任务 S1：解析本机 `userdata/{accountid}/config/localconfig.vdf` + `librarycache/{appid}/library_600x900.jpg`，做成无 Key、断网可用的 Steam 全库一键导入**——这才是用户要的「登录导入」。其余按本文 P1/P2 与 R2 推进。
