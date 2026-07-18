export type MediaType = "game" | "anime" | "comic" | "novel" | "video" | "tool";

export type SourceCapability =
  | "search"
  | "detail"
  | "chapters"
  | "roads"
  | "play"
  | "pages"
  | "text"
  | "download"
  | "webview"
  | "verify"
  | "metadata";

export type SourceLifecycle = "active" | "planned" | "reference";

export type SourceEcosystem =
  | "native"
  | "kazumi"
  | "tachiyomi"
  | "keiyoushi"
  | "aniyomi"
  | "kotatsu"
  | "cloudstream"
  | "suwayomi"
  | "komga"
  | "lanraragi"
  | "kavita"
  | "mangayomi"
  | "paperback"
  | "mangadex"
  | "gutendex"
  | "mediawiki"
  | "vndb"
  | "bangumi"
  | "ymgal"
  | "kungal"
  | "dlsite"
  | "getchu"
  | "erogamescape"
  | "itchio"
  | "igdb"
  | "external";

export type SourceAdoptionStrategy =
  | "ship"
  | "sync-rules"
  | "import-index"
  | "study-contract"
  | "manual-adapter";

export type SourceLicenseRisk = "low" | "medium" | "high" | "unknown";
export type SourceConnectorKind = "native" | "index" | "runtime" | "public-api" | "reference";
export type ExtensionIndexFormat = "tachiyomi" | "mangayomi" | "paperback" | "custom";
export type SourceAuthMode = "none" | "token" | "basic" | "api-key" | "session";
export type SourceNsfwPolicy = "hide-by-default" | "safe-only" | "user-controlled" | "unknown";

export interface SourceAdapterManifest {
  id: string;
  name: string;
  mediaType: MediaType;
  lifecycle: SourceLifecycle;
  ecosystem: SourceEcosystem;
  adoptionStrategy: SourceAdoptionStrategy;
  licenseRisk: SourceLicenseRisk;
  version: string;
  homepage?: string;
  referenceUrl?: string;
  referenceName?: string;
  license?: string;
  upstreamSourceCount?: number;
  indexUrl?: string;
  connectorKind?: SourceConnectorKind;
  indexFormat?: ExtensionIndexFormat;
  authMode?: SourceAuthMode;
  nsfwPolicy?: SourceNsfwPolicy;
  runtimeRequired?: boolean;
  capabilities: SourceCapability[];
  requiresVerification?: boolean;
  note: string;
}

export const MEDIA_TYPE_LABELS: Record<MediaType, string> = {
  game: "游戏",
  anime: "番剧",
  comic: "漫画",
  novel: "小说",
  video: "视频",
  tool: "工具",
};

export const CAPABILITY_LABELS: Record<SourceCapability, string> = {
  search: "搜索",
  detail: "详情",
  chapters: "章节",
  roads: "线路",
  play: "播放",
  pages: "图片",
  text: "正文",
  download: "下载",
  webview: "网页",
  verify: "验证",
  metadata: "元数据",
};

export const SOURCE_ECOSYSTEM_LABELS: Record<SourceEcosystem, string> = {
  native: "本地原生",
  kazumi: "Kazumi 规则",
  tachiyomi: "Tachiyomi / Mihon",
  keiyoushi: "Keiyoushi 扩展",
  aniyomi: "Aniyomi",
  kotatsu: "Kotatsu",
  cloudstream: "CloudStream",
  suwayomi: "Suwayomi",
  komga: "Komga",
  lanraragi: "LANraragi",
  kavita: "Kavita",
  mangayomi: "Mangayomi",
  paperback: "Paperback",
  mangadex: "MangaDex",
  gutendex: "Gutendex",
  mediawiki: "MediaWiki",
  vndb: "VNDB",
  bangumi: "Bangumi",
  ymgal: "月幕Galgame",
  kungal: "KUN Galgame",
  dlsite: "DLsite",
  getchu: "Getchu",
  erogamescape: "ErogameScape",
  itchio: "itch.io",
  igdb: "IGDB",
  external: "外部网页",
};

export const SOURCE_ADOPTION_LABELS: Record<SourceAdoptionStrategy, string> = {
  ship: "内置实现",
  "sync-rules": "同步规则",
  "import-index": "导入索引",
  "study-contract": "参考契约",
  "manual-adapter": "手写适配",
};

export const LICENSE_RISK_LABELS: Record<SourceLicenseRisk, string> = {
  low: "低风险",
  medium: "需隔离",
  high: "仅参考",
  unknown: "待确认",
};

export const SOURCE_ADAPTER_MANIFESTS: SourceAdapterManifest[] = [
  {
    id: "gutenberg-gutendex",
    name: "Project Gutenberg / Gutendex",
    mediaType: "novel",
    lifecycle: "active",
    ecosystem: "gutendex",
    adoptionStrategy: "ship",
    licenseRisk: "low",
    version: "0.13.9",
    homepage: "https://www.gutenberg.org",
    referenceUrl: "https://github.com/garethbjohnson/gutendex",
    referenceName: "garethbjohnson/gutendex",
    license: "MIT",
    connectorKind: "public-api",
    authMode: "none",
    nsfwPolicy: "safe-only",
    runtimeRequired: false,
    capabilities: ["search", "detail", "chapters", "text", "download", "metadata"],
    note: "通过 Gutendex 搜索 Project Gutenberg 目录，正文只读取上游公开文本，并提供上游 EPUB 下载。",
  },
  {
    id: "zh-wikisource-mediawiki",
    name: "中文维基文库",
    mediaType: "novel",
    lifecycle: "active",
    ecosystem: "mediawiki",
    adoptionStrategy: "ship",
    licenseRisk: "low",
    version: "0.13.9",
    homepage: "https://zh.wikisource.org",
    referenceUrl: "https://www.mediawiki.org/wiki/API:Main_page",
    referenceName: "MediaWiki Action API",
    connectorKind: "public-api",
    authMode: "none",
    nsfwPolicy: "safe-only",
    runtimeRequired: false,
    capabilities: ["search", "detail", "chapters", "text", "metadata"],
    note: "使用 MediaWiki 官方 API 搜索中文自由文本、识别目录并读取章节；许可信息以源站条目为准。",
  },
  {
    id: "kazumi-rules",
    name: "KazumiRules",
    mediaType: "anime",
    lifecycle: "active",
    ecosystem: "kazumi",
    adoptionStrategy: "sync-rules",
    licenseRisk: "low",
    version: "0.11.8",
    homepage: "https://github.com/Predidit/KazumiRules",
    referenceUrl: "https://github.com/Predidit/KazumiRules",
    referenceName: "Predidit/KazumiRules",
    license: "MIT",
    upstreamSourceCount: 100,
    connectorKind: "index",
    authMode: "none",
    nsfwPolicy: "unknown",
    capabilities: ["search", "detail", "roads", "play", "webview", "verify", "download"],
    requiresVerification: true,
    note: "当前番剧规则主线，继续扩展健康度排序、反爬验证和网页兜底。",
  },
  {
    id: "picacg-current",
    name: "PicACG",
    mediaType: "comic",
    lifecycle: "active",
    ecosystem: "native",
    adoptionStrategy: "ship",
    licenseRisk: "unknown",
    version: "0.12.0",
    connectorKind: "native",
    authMode: "session",
    nsfwPolicy: "user-controlled",
    capabilities: ["search", "detail", "chapters", "pages", "metadata"],
    requiresVerification: true,
    note: "0.12.0 保留独立 PicACG 成人入口，与普通漫画多源搜索和统一阅读器隔离。",
  },
  {
    id: "baozi-native",
    name: "包子漫画",
    mediaType: "comic",
    lifecycle: "active",
    ecosystem: "external",
    adoptionStrategy: "ship",
    licenseRisk: "medium",
    version: "0.12.0",
    homepage: "https://cn.baozimhcn.com",
    referenceUrl: "https://github.com/youniaogu/MangaReader",
    referenceName: "youniaogu/MangaReader Baozi adapter",
    license: "MIT",
    connectorKind: "native",
    authMode: "none",
    nsfwPolicy: "safe-only",
    runtimeRequired: false,
    capabilities: ["search", "detail", "chapters", "pages"],
    requiresVerification: true,
    note: "0.12.0 内置中文图片源；由 MoePlay 独立实现解析器，支持分源错误隔离。",
  },
  {
    id: "dm5-web-sources",
    name: "DM5 / 1kkk",
    mediaType: "comic",
    lifecycle: "active",
    ecosystem: "external",
    adoptionStrategy: "ship",
    licenseRisk: "medium",
    version: "0.12.0",
    connectorKind: "native",
    authMode: "none",
    nsfwPolicy: "safe-only",
    runtimeRequired: false,
    upstreamSourceCount: 2,
    capabilities: ["search", "detail", "chapters", "webview"],
    requiresVerification: true,
    note: "0.12.0 已内置两个动漫屋系网页源，阅读阶段使用受限 iframe 网页模式。",
  },
  {
    id: "tachiyomi-mihon-model",
    name: "Tachiyomi / Mihon Extensions",
    mediaType: "comic",
    lifecycle: "reference",
    ecosystem: "tachiyomi",
    adoptionStrategy: "import-index",
    licenseRisk: "low",
    version: "draft",
    homepage: "https://github.com/tachiyomiorg/extensions",
    referenceUrl: "https://github.com/mihonapp/mihon",
    referenceName: "tachiyomiorg/extensions + mihonapp/mihon",
    license: "Apache-2.0",
    indexUrl: "https://raw.githubusercontent.com/tachiyomiorg/extensions/repo/index.min.json",
    connectorKind: "index",
    indexFormat: "tachiyomi",
    authMode: "none",
    nsfwPolicy: "hide-by-default",
    runtimeRequired: true,
    capabilities: ["search", "detail", "chapters", "pages", "download"],
    note: "参考 source extension 能力拆分，不直接复制 Android/Kotlin 实现。",
  },
  {
    id: "keiyoushi-extensions",
    name: "Keiyoushi Extensions",
    mediaType: "comic",
    lifecycle: "reference",
    ecosystem: "keiyoushi",
    adoptionStrategy: "import-index",
    licenseRisk: "low",
    version: "draft",
    homepage: "https://github.com/keiyoushi/extensions",
    referenceUrl: "https://github.com/keiyoushi/extensions",
    referenceName: "keiyoushi/extensions",
    license: "Apache-2.0",
    indexUrl: "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json",
    connectorKind: "index",
    indexFormat: "tachiyomi",
    authMode: "none",
    nsfwPolicy: "hide-by-default",
    runtimeRequired: true,
    capabilities: ["search", "detail", "chapters", "pages", "download"],
    note: "当前活跃的 Mihon/Tachiyomi 扩展索引；只读导入源目录，不执行 APK。",
  },
  {
    id: "suwayomi-runtime",
    name: "Suwayomi Runtime",
    mediaType: "comic",
    lifecycle: "planned",
    ecosystem: "suwayomi",
    adoptionStrategy: "manual-adapter",
    licenseRisk: "medium",
    version: "draft",
    homepage: "https://github.com/Suwayomi/Suwayomi-Server",
    referenceUrl: "https://github.com/Suwayomi/Suwayomi-Server",
    referenceName: "Suwayomi/Suwayomi-Server",
    license: "MPL-2.0",
    connectorKind: "runtime",
    authMode: "token",
    nsfwPolicy: "hide-by-default",
    runtimeRequired: true,
    capabilities: ["search", "detail", "chapters", "pages", "download", "metadata"],
    note: "外部本地运行时优先接入对象，用 GraphQL 读取已安装扩展和源，不内嵌服务端。",
  },
  {
    id: "komga-runtime",
    name: "Komga",
    mediaType: "comic",
    lifecycle: "planned",
    ecosystem: "komga",
    adoptionStrategy: "manual-adapter",
    licenseRisk: "low",
    version: "draft",
    homepage: "https://github.com/gotson/komga",
    referenceUrl: "https://github.com/gotson/komga",
    referenceName: "gotson/komga",
    license: "MIT",
    connectorKind: "runtime",
    authMode: "basic",
    nsfwPolicy: "user-controlled",
    runtimeRequired: true,
    capabilities: ["search", "detail", "chapters", "pages", "download", "metadata"],
    note: "自托管漫画/图书服务器，优先通过外部 API/OPDS 读取个人库。",
  },
  {
    id: "lanraragi-runtime",
    name: "LANraragi",
    mediaType: "comic",
    lifecycle: "planned",
    ecosystem: "lanraragi",
    adoptionStrategy: "manual-adapter",
    licenseRisk: "low",
    version: "draft",
    homepage: "https://github.com/Difegue/LANraragi",
    referenceUrl: "https://github.com/Difegue/LANraragi",
    referenceName: "Difegue/LANraragi",
    license: "MIT",
    connectorKind: "runtime",
    authMode: "api-key",
    nsfwPolicy: "hide-by-default",
    runtimeRequired: true,
    capabilities: ["search", "detail", "chapters", "pages", "metadata"],
    note: "自托管漫画库，适合通过外部 API/OPDS 接入本地收藏。",
  },
  {
    id: "kavita-runtime",
    name: "Kavita",
    mediaType: "comic",
    lifecycle: "planned",
    ecosystem: "kavita",
    adoptionStrategy: "manual-adapter",
    licenseRisk: "high",
    version: "draft",
    homepage: "https://github.com/Kareadita/Kavita",
    referenceUrl: "https://github.com/Kareadita/Kavita",
    referenceName: "Kareadita/Kavita",
    license: "GPL-3.0",
    connectorKind: "runtime",
    authMode: "token",
    nsfwPolicy: "user-controlled",
    runtimeRequired: true,
    capabilities: ["search", "detail", "chapters", "pages", "metadata"],
    note: "仅作为外部服务器 API 边界接入，不复制 GPL 实现进 MoePlay。",
  },
  {
    id: "yuzono-tachiyomi-model",
    name: "Yuzono Extensions",
    mediaType: "comic",
    lifecycle: "reference",
    ecosystem: "tachiyomi",
    adoptionStrategy: "study-contract",
    licenseRisk: "low",
    version: "draft",
    homepage: "https://github.com/yuzono/tachiyomi-extensions",
    referenceUrl: "https://github.com/yuzono/tachiyomi-extensions",
    referenceName: "yuzono/tachiyomi-extensions",
    license: "Apache-2.0",
    connectorKind: "reference",
    authMode: "none",
    nsfwPolicy: "hide-by-default",
    runtimeRequired: true,
    capabilities: ["search", "detail", "chapters", "pages", "download"],
    note: "活跃 Mihon/Tachiyomi fork，当前没有标准 repo 索引分支，先参考模块组织和源覆盖。",
  },
  {
    id: "kotatsu-parser-model",
    name: "Kotatsu Parsers",
    mediaType: "comic",
    lifecycle: "reference",
    ecosystem: "kotatsu",
    adoptionStrategy: "study-contract",
    licenseRisk: "high",
    version: "draft",
    homepage: "https://github.com/KotatsuApp/kotatsu-parsers",
    referenceUrl: "https://github.com/KotatsuApp/kotatsu-parsers",
    referenceName: "KotatsuApp/kotatsu-parsers",
    license: "GPL-3.0",
    upstreamSourceCount: 1256,
    connectorKind: "reference",
    authMode: "none",
    nsfwPolicy: "hide-by-default",
    runtimeRequired: true,
    capabilities: ["search", "detail", "chapters", "pages", "download"],
    note: "覆盖漫画源很多，但 GPL 代码仅做架构与站点清单参考，不直接并入。",
  },
  {
    id: "aniyomi-model",
    name: "Aniyomi Extensions",
    mediaType: "video",
    lifecycle: "reference",
    ecosystem: "aniyomi",
    adoptionStrategy: "import-index",
    licenseRisk: "low",
    version: "draft",
    homepage: "https://github.com/aniyomiorg/aniyomi-extensions",
    referenceUrl: "https://github.com/aniyomiorg/aniyomi",
    referenceName: "aniyomiorg/aniyomi + aniyomi-extensions",
    license: "Apache-2.0",
    indexUrl: "https://raw.githubusercontent.com/aniyomiorg/aniyomi-extensions/repo/index.min.json",
    connectorKind: "index",
    indexFormat: "tachiyomi",
    authMode: "none",
    nsfwPolicy: "hide-by-default",
    runtimeRequired: true,
    capabilities: ["search", "detail", "chapters", "play", "webview", "download"],
    requiresVerification: true,
    note: "参考动漫与漫画统一扩展体验，映射到 MoePlay 的播放和网页兜底。",
  },
  {
    id: "mangayomi-extensions",
    name: "Mangayomi Extensions",
    mediaType: "comic",
    lifecycle: "reference",
    ecosystem: "mangayomi",
    adoptionStrategy: "import-index",
    licenseRisk: "low",
    version: "draft",
    homepage: "https://github.com/kodjodevf/mangayomi-extensions",
    referenceUrl: "https://github.com/kodjodevf/mangayomi",
    referenceName: "kodjodevf/mangayomi + mangayomi-extensions",
    license: "Apache-2.0",
    indexUrl: "https://raw.githubusercontent.com/kodjodevf/mangayomi-extensions/main/index.json",
    connectorKind: "index",
    indexFormat: "mangayomi",
    authMode: "none",
    nsfwPolicy: "hide-by-default",
    runtimeRequired: true,
    capabilities: ["search", "detail", "chapters", "pages", "download"],
    note: "跨平台漫画/小说扩展索引，先归一元数据，不执行 Dart/JS 插件。",
  },
  {
    id: "paperback-extensions",
    name: "Paperback Extensions",
    mediaType: "comic",
    lifecycle: "reference",
    ecosystem: "paperback",
    adoptionStrategy: "study-contract",
    licenseRisk: "unknown",
    version: "draft",
    homepage: "https://github.com/Paperback-iOS/app",
    referenceUrl: "https://github.com/Paperback-iOS/app",
    referenceName: "Paperback-iOS/app",
    connectorKind: "reference",
    indexFormat: "paperback",
    authMode: "none",
    nsfwPolicy: "hide-by-default",
    runtimeRequired: true,
    capabilities: ["search", "detail", "chapters", "pages"],
    note: "iOS 插件生态仅做契约参考，暂不执行第三方插件。",
  },
  {
    id: "mangadex-api",
    name: "MangaDex API",
    mediaType: "comic",
    lifecycle: "active",
    ecosystem: "mangadex",
    adoptionStrategy: "ship",
    licenseRisk: "medium",
    version: "0.12.0",
    homepage: "https://api.mangadex.org",
    referenceUrl: "https://api.mangadex.org/docs/",
    referenceName: "MangaDex API",
    connectorKind: "public-api",
    authMode: "none",
    nsfwPolicy: "safe-only",
    runtimeRequired: false,
    capabilities: ["search", "detail", "chapters", "pages", "metadata"],
    note: "0.12.0 已接入公开 API 搜索、详情、章节和图片阅读，并纳入并行聚合搜索。",
  },
  {
    id: "cloudstream-model",
    name: "CloudStream Extensions",
    mediaType: "video",
    lifecycle: "reference",
    ecosystem: "cloudstream",
    adoptionStrategy: "study-contract",
    licenseRisk: "high",
    version: "draft",
    homepage: "https://github.com/recloudstream/cloudstream",
    referenceUrl: "https://github.com/recloudstream/cloudstream-extensions",
    referenceName: "recloudstream/cloudstream + cloudstream-extensions",
    license: "GPL-3.0",
    connectorKind: "reference",
    authMode: "none",
    nsfwPolicy: "unknown",
    runtimeRequired: true,
    capabilities: ["search", "detail", "play", "webview", "download"],
    requiresVerification: true,
    note: "视频与动漫插件生态参考；仅研究站点能力和外部运行时边界，不直接复制实现。",
  },
  {
    id: "external-video-open",
    name: "外部视频网站",
    mediaType: "video",
    lifecycle: "planned",
    ecosystem: "external",
    adoptionStrategy: "manual-adapter",
    licenseRisk: "medium",
    version: "draft",
    connectorKind: "reference",
    authMode: "none",
    nsfwPolicy: "unknown",
    capabilities: ["detail", "play", "webview"],
    note: "先按可提取、可网页、仅外部打开三类能力分层，不绕过付费或加密限制。",
  },
  {
    id: "vndb-kana-api",
    name: "VNDB Kana API",
    mediaType: "game",
    lifecycle: "active",
    ecosystem: "vndb",
    adoptionStrategy: "ship",
    licenseRisk: "low",
    version: "0.13.9",
    homepage: "https://vndb.org",
    referenceUrl: "https://api.vndb.org/kana",
    referenceName: "VNDB.org API v2 (Kana)",
    connectorKind: "public-api",
    authMode: "none",
    nsfwPolicy: "user-controlled",
    runtimeRequired: false,
    capabilities: ["search", "detail", "metadata", "webview"],
    note: "已内置视觉小说搜索与详情抓取，移动端仅展示公开元数据和外部网页，不提供游戏下载。",
  },
  {
    id: "bangumi-game-api",
    name: "Bangumi 游戏条目",
    mediaType: "game",
    lifecycle: "active",
    ecosystem: "bangumi",
    adoptionStrategy: "ship",
    licenseRisk: "low",
    version: "0.13.9",
    homepage: "https://bgm.tv/game",
    referenceUrl: "https://github.com/bangumi/api",
    referenceName: "bangumi/api",
    connectorKind: "public-api",
    authMode: "none",
    nsfwPolicy: "user-controlled",
    runtimeRequired: false,
    capabilities: ["search", "detail", "metadata", "webview"],
    note: "用于补全中文别名、评分、标签和简介；只保存条目资料与跳转链接。",
  },
  {
    id: "ymgal-api",
    name: "月幕Galgame",
    mediaType: "game",
    lifecycle: "active",
    ecosystem: "ymgal",
    adoptionStrategy: "ship",
    licenseRisk: "medium",
    version: "0.13.9",
    homepage: "https://www.ymgal.games",
    referenceUrl: "https://www.ymgal.games",
    referenceName: "Ymgal public endpoints",
    connectorKind: "public-api",
    authMode: "none",
    nsfwPolicy: "user-controlled",
    runtimeRequired: false,
    capabilities: ["search", "detail", "metadata", "webview"],
    requiresVerification: true,
    note: "已接入公开搜索和详情端点，作为中文 galgame 元数据补全源；不聚合下载地址。",
  },
  {
    id: "kungal-metadata",
    name: "KUN Galgame",
    mediaType: "game",
    lifecycle: "active",
    ecosystem: "kungal",
    adoptionStrategy: "ship",
    licenseRisk: "medium",
    version: "0.13.9",
    homepage: "https://www.kungal.com",
    referenceUrl: "https://www.kungal.com",
    referenceName: "KUN Galgame public pages",
    connectorKind: "reference",
    authMode: "none",
    nsfwPolicy: "user-controlled",
    runtimeRequired: false,
    capabilities: ["search", "detail", "metadata", "webview"],
    requiresVerification: true,
    note: "仅用于公开页面搜索、条目识别和资料跳转；涉及资源帖时保持网页外链，不在应用内做下载分发。",
  },
  {
    id: "dlsite-store-metadata",
    name: "DLsite 正版商店资料",
    mediaType: "game",
    lifecycle: "active",
    ecosystem: "dlsite",
    adoptionStrategy: "ship",
    licenseRisk: "medium",
    version: "0.13.9",
    homepage: "https://www.dlsite.com",
    referenceUrl: "https://www.dlsite.com",
    referenceName: "DLsite product pages",
    connectorKind: "reference",
    authMode: "none",
    nsfwPolicy: "user-controlled",
    runtimeRequired: false,
    capabilities: ["search", "detail", "metadata", "webview"],
    requiresVerification: true,
    note: "作为正版商店页面元数据源，保留年龄分级和用户开关，不抓取付费内容。",
  },
  {
    id: "getchu-store-metadata",
    name: "Getchu 商店资料",
    mediaType: "game",
    lifecycle: "active",
    ecosystem: "getchu",
    adoptionStrategy: "ship",
    licenseRisk: "medium",
    version: "0.13.9",
    homepage: "https://www.getchu.com",
    referenceUrl: "https://www.getchu.com",
    referenceName: "Getchu product pages",
    connectorKind: "reference",
    authMode: "none",
    nsfwPolicy: "hide-by-default",
    runtimeRequired: false,
    capabilities: ["search", "detail", "metadata", "webview"],
    requiresVerification: true,
    note: "已有可选元数据抓取，默认按成人内容隔离；仅提取商品资料，不绕过站点限制。",
  },
  {
    id: "erogamescape-metadata",
    name: "ErogameScape 评分资料",
    mediaType: "game",
    lifecycle: "active",
    ecosystem: "erogamescape",
    adoptionStrategy: "ship",
    licenseRisk: "medium",
    version: "0.13.9",
    homepage: "https://erogamescape.org",
    referenceUrl: "https://erogamescape.org/~ap2/ero/toukei_kaiseki/",
    referenceName: "ErogameScape public pages",
    connectorKind: "reference",
    authMode: "none",
    nsfwPolicy: "user-controlled",
    runtimeRequired: false,
    capabilities: ["search", "detail", "metadata", "webview"],
    requiresVerification: true,
    note: "面向日文商业 galgame 的评分与发售信息补全，网络不稳定时降级为外部网页。",
  },
  {
    id: "itchio-visual-novel-store",
    name: "itch.io 视觉小说商店",
    mediaType: "game",
    lifecycle: "planned",
    ecosystem: "itchio",
    adoptionStrategy: "manual-adapter",
    licenseRisk: "low",
    version: "draft",
    homepage: "https://itch.io/games/tag-visual-novel",
    referenceUrl: "https://github.com/itchio/itch",
    referenceName: "itchio/itch",
    license: "MIT",
    connectorKind: "public-api",
    authMode: "api-key",
    nsfwPolicy: "user-controlled",
    runtimeRequired: false,
    capabilities: ["search", "detail", "metadata", "webview"],
    note: "参考开源 itch 客户端和官方 API，后续只接入用户已拥有或免费作品的正版商店资料与外部打开。",
  },
  {
    id: "igdb-game-metadata",
    name: "IGDB 游戏资料库",
    mediaType: "game",
    lifecycle: "planned",
    ecosystem: "igdb",
    adoptionStrategy: "manual-adapter",
    licenseRisk: "low",
    version: "draft",
    homepage: "https://www.igdb.com",
    referenceUrl: "https://api-docs.igdb.com",
    referenceName: "IGDB API docs",
    connectorKind: "public-api",
    authMode: "api-key",
    nsfwPolicy: "safe-only",
    runtimeRequired: false,
    capabilities: ["search", "detail", "metadata", "webview"],
    note: "作为泛游戏元数据补全候选，需要用户配置 Twitch/IGDB 凭据后启用。",
  },
  {
    id: "local-game-library",
    name: "本地游戏库",
    mediaType: "game",
    lifecycle: "active",
    ecosystem: "native",
    adoptionStrategy: "ship",
    licenseRisk: "low",
    version: "0.11.8",
    connectorKind: "native",
    authMode: "none",
    nsfwPolicy: "user-controlled",
    capabilities: ["detail", "metadata", "download"],
    note: "已有本地导入、启动、刮削和统计能力，后续接入统一任务中心。",
  },
];

export function getSourceAdaptersByMediaType(mediaType: MediaType): SourceAdapterManifest[] {
  return SOURCE_ADAPTER_MANIFESTS.filter((source) => source.mediaType === mediaType);
}

export function getSourceAdaptersByLifecycle(lifecycle: SourceLifecycle): SourceAdapterManifest[] {
  return SOURCE_ADAPTER_MANIFESTS.filter((source) => source.lifecycle === lifecycle);
}

export function getSourceAdaptersByEcosystem(ecosystem: SourceEcosystem): SourceAdapterManifest[] {
  return SOURCE_ADAPTER_MANIFESTS.filter((source) => source.ecosystem === ecosystem);
}

export function getSourceAdaptersReadyForIndexImport(): SourceAdapterManifest[] {
  return SOURCE_ADAPTER_MANIFESTS.filter(
    (source) => source.adoptionStrategy === "import-index" && Boolean(source.indexUrl),
  );
}

export function getSourceAdapterSummary(sources: SourceAdapterManifest[] = SOURCE_ADAPTER_MANIFESTS) {
  const byMediaType = Object.fromEntries(
    (Object.keys(MEDIA_TYPE_LABELS) as MediaType[]).map((mediaType) => [
      mediaType,
      sources.filter((source) => source.mediaType === mediaType).length,
    ]),
  ) as Record<MediaType, number>;

  return {
    total: sources.length,
    active: sources.filter((source) => source.lifecycle === "active").length,
    planned: sources.filter((source) => source.lifecycle === "planned").length,
    references: sources.filter((source) => source.lifecycle === "reference").length,
    requiresVerification: sources.filter((source) => source.requiresVerification).length,
    indexImportable: sources.filter((source) => source.adoptionStrategy === "import-index" && source.indexUrl).length,
    highLicenseRisk: sources.filter((source) => source.licenseRisk === "high").length,
    byMediaType,
  };
}
