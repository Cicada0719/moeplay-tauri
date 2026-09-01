// api/rules 规则列表缓存测试：加载命令只在初始化或显式刷新时触发。
import { beforeEach, describe, expect, it } from "vitest";
import { clearMockInvokeHandler, mockRouter, setMockInvokeHandler } from "./core";
import {
  checkAndUpdateRules,
  getHealth,
  getLoadedRules,
  getRulesMeta,
  importRule,
  probeHealth,
  refreshLoadedRules,
  removeCustomRule,
  type LoadedRule,
} from "./rules";

const READY_RULE: LoadedRule = {
  id: "rule-a",
  manifest: {
    name: "源A",
    version: "1.0.0",
    contentType: "anime",
    baseUrl: "https://example.com",
    language: "zh-CN",
    nsfw: false,
    author: null,
    search: "function search(k,p){ return []; }",
    detail: "function detail(u){ return {}; }",
    chapter: "function chapter(u){ return []; }",
    parse: "function parse(u){ return { urls: [], kind: 'video' }; }",
  },
  origin: "builtin",
  status: "ready",
  error: null,
};

describe("api/rules 规则缓存", () => {
  beforeEach(() => {
    clearMockInvokeHandler();
  });

  it("getLoadedRules 只触发一次 rules_load_all，后续命中缓存", async () => {
    let calls = 0;
    setMockInvokeHandler(
      mockRouter({
        rules_load_all: () => {
          calls++;
          return [READY_RULE];
        },
      }),
    );

    const first = await getLoadedRules();
    const second = await getLoadedRules();
    const third = await getLoadedRules();

    expect(calls).toBe(1); // 加载命令只在首次访问时触发（重扫目录 + compile_check 开销只付一次）
    expect(first).toEqual(second);
    expect(second).toEqual(third);
  });

  it("refreshLoadedRules 显式刷新后重新触发加载", async () => {
    let calls = 0;
    setMockInvokeHandler(
      mockRouter({
        rules_load_all: () => {
          calls++;
          return [READY_RULE];
        },
      }),
    );

    // 模块级缓存可能被前序用例填充；refresh 语义是「清缓存 + 重新触发加载命令」，
    // 无论缓存当前是否有值，每次显式刷新都应新增一次加载调用。
    const before = calls;
    await refreshLoadedRules();
    expect(calls).toBe(before + 1);
    await refreshLoadedRules();
    expect(calls).toBe(before + 2);
  });

  it("importRule 成功后失效缓存并后台重载，下次 getLoadedRules 拿到最新列表", async () => {
    let calls = 0;
    setMockInvokeHandler(
      mockRouter({
        rules_load_all: () => {
          calls++;
          return [READY_RULE];
        },
        rules_import: () => ({ ...READY_RULE, id: "rule-new", origin: "custom" }),
      }),
    );

    // 先确保缓存就绪（命中或首次加载均接受），再从该基准计数
    await getLoadedRules();
    const afterWarm = calls;

    // 导入成功 → API 内部 void refreshLoadedRules：缓存失效 + 后台触发一次重载
    await importRule("C:\\rules\\new.json");
    expect(calls).toBe(afterWarm + 1);

    // 之后的 getLoadedRules 命中后台重载结果，不重复触发加载命令
    await getLoadedRules();
    await getLoadedRules();
    expect(calls).toBe(afterWarm + 1);
  });

  it("removeCustomRule 成功后失效缓存并后台重载", async () => {
    let calls = 0;
    setMockInvokeHandler(
      mockRouter({
        rules_load_all: () => {
          calls++;
          return [READY_RULE];
        },
        rules_remove_custom: () => undefined,
      }),
    );

    await getLoadedRules();
    const afterWarm = calls;

    await removeCustomRule("rule-a");
    expect(calls).toBe(afterWarm + 1);

    await getLoadedRules();
    expect(calls).toBe(afterWarm + 1);
  });

  it("importRule 失败不清空缓存（既有列表保持可用）", async () => {
    let calls = 0;
    setMockInvokeHandler(
      mockRouter({
        rules_load_all: () => {
          calls++;
          return [READY_RULE];
        },
        rules_import: () => {
          throw new Error("规则缺少必需字段: parse");
        },
      }),
    );

    await getLoadedRules();
    const afterWarm = calls;

    await expect(importRule("C:\\rules\\bad.json")).rejects.toThrow("parse");
    // 失败路径不触发刷新：缓存仍是导入前的列表
    expect(calls).toBe(afterWarm);
    const rules = await getLoadedRules();
    expect(rules).toHaveLength(1);
  });

  it("加载失败后重置缓存，下一次 getLoadedRules 重新发起加载可自愈", async () => {
    let calls = 0;
    setMockInvokeHandler(
      mockRouter({
        rules_load_all: () => {
          calls++;
          if (calls === 1) throw new Error("网络中断");
          return [READY_RULE];
        },
      }),
    );

    // refresh 重置模块级缓存（可能被前序用例填充），并作为第一次加载（失败）。
    // 失败不缓存 rejected Promise：await 必须 reject，而不是返回永久的失败 Promise。
    await expect(refreshLoadedRules()).rejects.toThrow("网络中断");
    expect(calls).toBe(1);

    // 下一次调用重新发起加载（而非复用旧 rejected Promise），成功后命中缓存
    const rules = await getLoadedRules();
    expect(calls).toBe(2);
    expect(rules).toEqual([READY_RULE]);
    await getLoadedRules();
    expect(calls).toBe(2);
  });
});

// ── 规则包热更新 + 健康检查命令封装（spec task-02 §3.5）──────────────────

describe("api/rules 热更新与健康检查命令", () => {
  beforeEach(() => {
    clearMockInvokeHandler();
  });

  it("getRulesMeta 调用 rules_get_meta", async () => {
    const meta = {
      packageVersion: "2026.08.1",
      source: "bundled" as const,
      updatedAt: 1785830400,
      lastCheckAt: null,
      ruleCount: 13,
      remoteBase: "https://raw.githubusercontent.com/Cicada0719/moeplay-tauri-rules/main/",
    };
    const seen: unknown[] = [];
    setMockInvokeHandler(
      mockRouter({
        rules_get_meta: (cmd, args) => {
          seen.push(args);
          return meta;
        },
      }),
    );
    await expect(getRulesMeta()).resolves.toEqual(meta);
    expect(seen[0]).toEqual({});
  });

  it("checkAndUpdateRules 透传 force 参数（默认 false）", async () => {
    const seen: unknown[] = [];
    setMockInvokeHandler(
      mockRouter({
        rules_check_and_update: (_cmd, args) => {
          seen.push(args);
          return { status: "updated", fromVersion: "2026.08.1", toVersion: "2026.08.2", updatedRules: 13 };
        },
      }),
    );
    const outcome = await checkAndUpdateRules();
    expect(outcome.status).toBe("updated");
    await checkAndUpdateRules(true);
    expect(seen).toEqual([{ force: false }, { force: true }]);
  });

  it("probeHealth：无参 → sourceIds=null；有参 → 数组透传", async () => {
    const seen: unknown[] = [];
    setMockInvokeHandler(
      mockRouter({
        rules_probe_health: (_cmd, args) => {
          seen.push(args);
          return [];
        },
      }),
    );
    await probeHealth();
    await probeHealth(["a", "b"]);
    expect(seen).toEqual([{ sourceIds: null }, { sourceIds: ["a", "b"] }]);
  });

  it("getHealth 调用 rules_get_health 并返回健康列表", async () => {
    const list = [
      {
        sourceId: "agefans",
        status: "Healthy",
        consecutiveFailures: 0,
        lastCheckedAt: 1785830400,
        lastLatencyMs: 42,
        lastError: null,
      },
    ];
    setMockInvokeHandler(
      mockRouter({
        rules_get_health: () => list,
      }),
    );
    await expect(getHealth()).resolves.toEqual(list);
  });
});
