#!/usr/bin/env python3
"""kazumi 规则转换器：KazumiRules/*.json → MoeGame 内置规则（JSON 内嵌 JS 函数）。

用法: python scripts/gen_kazumi_rules.py <kazumi-rules目录> [输出目录]
默认输出到仓库 src-tauri/resources/rules/。
"""
import json
import os
import re
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ADAPTER = open(os.path.join(HERE, "kazumi-adapter.js"), encoding="utf-8").read()

TYPE_MAP = {"anime": "anime", "manga": "manga", "novel": "novel"}

def js_str(s):
    """JS 字符串字面量（转义 \\ 和 " 和换行）"""
    return json.dumps(s, ensure_ascii=False)

def build_rule(src: dict) -> dict | None:
    if src.get("deprecated"):
        return None
    if src.get("api") not in ("1", "2", "3", "4"):
        # api 5/6/7 为 JSON API 或 JS 脚本规则，格式差异大，暂不转换
        return None
    ctype = TYPE_MAP.get(str(src.get("type", "")).lower())
    if not ctype:
        return None
    base = src.get("baseURL", "").rstrip("/")
    if not base.startswith("http"):
        return None
    name = src.get("name", "").strip()
    if not name:
        return None
    rid = re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-") or "rule"
    search_url = src.get("searchURL", "").replace(" ", "")
    if not search_url or "@keyword" not in search_url:
        return None
    ua = src.get("userAgent") or "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36"
    referer = src.get("referer", "") or (base + "/")
    headers = {"User-Agent": ua, "Referer": referer}
    hdr = "{" + ", ".join(f"{js_str(k)}: {js_str(v)}" for k, v in headers.items()) + "}"

    sl, sn, sr = src.get("searchList", ""), src.get("searchName", ""), src.get("searchResult", "")
    cr, ce = src.get("chapterRoads", ""), src.get("chapterResult", "")
    if not (sl and sn and sr and cr and ce):
        return None

    search_fn = f"""function search(kw, page) {{
  try {{
    var url = {js_str(search_url)}.replace('@keyword', encodeURIComponent(kw));
    return fetch(url, {{ headers: {hdr} }}).then(function (r) {{
      if (r.status !== 200 || !r.dom) return [];
      var root = JSON.parse(r.dom);
      var list = kzXPath(root, {js_str(sl)});
      var out = [];
      for (var i = 0; i < list.length && i < 60; i++) {{
        var nameEl = kzXPath1(list[i], {js_str(sn)});
        var linkEl = kzXPath1(list[i], {js_str(sr)});
        var title = nameEl ? kzText(nameEl) : '';
        var href = kzHref(linkEl);
        if (!href) continue;
        if (!title) title = decodeURIComponent(href.split('/').pop() || '').replace(/\\.html?$/, '');
        out.push({{ title: title, url: kzAbs({js_str(base)}, href), cover: '', extra: {{}} }});
      }}
      return out;
    }}).catch(function (e) {{ throw new Error('search: ' + e.message); }});
  }} catch (e) {{ throw new Error('search: ' + e.message); }}
}}"""

    detail_fn = f"""function detail(url) {{
  return fetch(url, {{ headers: {hdr} }}).then(function (r) {{
    if (r.status !== 200) return {{ title: '未知', cover: null, description: null, extra: {{}} }};
    var html = r.body || '';
    var t = /<title[^>]*>([^<]+)/i.exec(html);
    var og = /<meta[^>]+property=["']og:image["'][^>]+content=["']([^"']+)/i.exec(html)
         || /<meta[^>]+content=["']([^"']+)["'][^>]+property=["']og:image["']/i.exec(html);
    return {{
      title: t ? t[1].trim().replace(/\\s*[-_|]\\s*(第\\d+[集话]|全集).*$/, '').trim() : '未知',
      cover: og ? og[1] : null,
      description: null,
      extra: {{}}
    }};
  }}).catch(function () {{ return {{ title: '未知', cover: null, description: null, extra: {{}} }}; }});
}}"""

    chapter_fn = f"""function chapter(detailUrl) {{
  return fetch(detailUrl, {{ headers: {hdr} }}).then(function (r) {{
    if (r.status !== 200 || !r.dom) return [];
    var root = JSON.parse(r.dom);
    var roads = kzXPath(root, {js_str(cr)});
    var out = [];
    var idx = 0;
    var seen = {{}};
    for (var ri = 0; ri < roads.length && ri < 8; ri++) {{
      var eps = kzXPath(roads[ri], {js_str(ce)});
      for (var ei = 0; ei < eps.length && idx < 500; ei++) {{
        var href = kzHref(eps[ei]);
        if (!href) continue;
        href = kzAbs({js_str(base)}, href);
        if (seen[href]) continue;
        seen[href] = true;
        idx++;
        var title = kzText(eps[ei]);
        if (!title) title = '第' + idx + '集';
        out.push({{ id: String(idx), title: title, url: href, index: idx }});
      }}
    }}
    return out;
  }}).catch(function (e) {{ throw new Error('chapter: ' + e.message); }});
}}"""

    parse_fn = f"""function parse(chapterUrl) {{
  return kzParsePage(chapterUrl, 0, {hdr});
}}"""

    rule = {
        "name": name,
        "id": rid,
        "version": "1.0.0",
        "contentType": ctype,
        "baseUrl": base + "/",
        "language": "zh-CN",
        "nsfw": False,
        "author": "KazumiRules (converted)",
        "probeKeyword": "进击的巨人" if ctype == "anime" else ("海贼王" if ctype == "manga" else "斗破苍穹"),
        "licenseNote": f"Converted from KazumiRules/{os.path.basename(src.get('_file', 'rule.json'))} (api={src.get('api')}). Original site may change structure; hot-update path is the remedy.",
        "search": search_fn,
        "detail": detail_fn,
        "chapter": chapter_fn,
        "parse": parse_fn,
        "_kazumi": {"api": src.get("api"), "original": src.get("_file", ""), "muliSources": src.get("muliSources", False)},
    }
    return rule

def main():
    src_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "..", "..", "..", "factory-work", "kazumi-rules")
    out_dir = sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, "..", "src-tauri", "resources", "rules")
    src_dir = os.path.abspath(src_dir)
    out_dir = os.path.abspath(out_dir)

    by_type = {"anime": [], "manga": [], "novel": []}
    skipped = []
    for fn in sorted(os.listdir(src_dir)):
        if not fn.endswith(".json"):
            continue
        try:
            raw = json.load(open(os.path.join(src_dir, fn), encoding="utf-8"))
        except Exception:
            skipped.append((fn, "parse-error"))
            continue
        raw["_file"] = fn
        rule = build_rule(raw)
        if rule is None:
            reason = "deprecated" if raw.get("deprecated") else (
                f"api={raw.get('api')}" if raw.get("api") not in ("1", "2", "3", "4") else
                f"type={raw.get('type')}" if TYPE_MAP.get(str(raw.get("type", "")).lower()) is None else
                "missing-fields")
            skipped.append((fn, reason))
            continue
        by_type[rule["contentType"]].append(rule)

    for ctype, rules in by_type.items():
        d = os.path.join(out_dir, ctype)
        os.makedirs(d, exist_ok=True)
        # 清掉旧的转换产物（保留手写规则：无 _kazumi 字段的）
        for old in os.listdir(d):
            if not old.endswith(".json"):
                continue
            try:
                old_data = json.load(open(os.path.join(d, old), encoding="utf-8"))
            except Exception:
                continue
            if old_data.get("_kazumi"):
                os.remove(os.path.join(d, old))
        for rule in rules:
            payload = {k: v for k, v in rule.items() if not k.startswith("_")}
            payload["search"] = ADAPTER + "\n\n" + payload["search"]
            payload["chapter"] = ADAPTER + "\n\n" + payload["chapter"]
            payload["detail"] = ADAPTER + "\n\n" + payload["detail"]
            payload["parse"] = ADAPTER + "\n\n" + payload["parse"]
            out = os.path.join(d, rule["id"] + ".json")
            json.dump(payload, open(out, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
            print(f"  [{ctype}] {rule['name']} -> {rule['id']}.json")

    total = sum(len(v) for v in by_type.values())
    print(f"\n生成 {total} 条规则 (anime={len(by_type['anime'])}, manga={len(by_type['manga'])}, novel={len(by_type['novel'])})")
    print(f"跳过 {len(skipped)} 条: {skipped[:12]}")

if __name__ == "__main__":
    main()
