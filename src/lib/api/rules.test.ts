// api/rules 规则列表缓存测试（Kimi K3 复审第 1 项：加载命令只在初始化或显式刷新时触发）
import { beforeEach, describe, expect, it } from "vitest";
import { clearMockInvokeHandler, mockRouter, setMockInvokeHandler } from "./core";
import { getLoadedRules, importRule, refreshLoadedRules, removeCustomRule, type LoadedRule } from "./rules";

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

  it("importRule 成功后失效缓存并后台重载，下次 getLoadedRules 拿到最新列表（Kimi K3 复审第 7 项）", async () => {
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

  it("removeCustomRule 成功后失效缓存并后台重载（Kimi K3 复审第 7 项）", async () => {
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
});
