// ============================================================
// kazumi 规则适配层（内嵌于每个由 KazumiRules 转换的规则文件）
// DOM 结构（Rust scraper 解析，JSON.parse(r.dom)）：
//   元素: {t: 标签, a: {属性}, c: [子节点]}
//   文本: {x: 文本}
// ============================================================

function kzParse(expr) {
  var s = (expr || '').trim();
  if (s.indexOf('//') === 0) s = s.slice(2);
  else if (s.indexOf('/') === 0) s = s.slice(1);
  var steps = [];
  s.split('/').forEach(function (seg) {
    seg = seg.trim();
    if (!seg) return;
    if (seg === 'text()') { steps.push({ text: true }); return; }
    var m = /^([a-zA-Z0-9_*]+)(?:\[(\d+)\])?$/.exec(seg);
    if (!m) return;
    steps.push({ tag: m[1].toLowerCase(), idx: m[2] ? parseInt(m[2], 10) : 0 });
  });
  return steps;
}

function kzChildren(node) { return node && node.c ? node.c : []; }
function kzTag(node) { return node && node.t ? node.t.toLowerCase() : ''; }

// 在 subtree 内按 step 匹配（兄弟组语义：每个父节点下第 idx 个 tag 匹配子节点）
function kzStep(subtree, step) {
  var out = [];
  var stack = [subtree];
  while (stack.length) {
    var n = stack.pop();
    var cs = kzChildren(n);
    var cnt = 0;
    for (var i = 0; i < cs.length; i++) {
      var c = cs[i];
      if (step.tag === '*' || kzTag(c) === step.tag) {
        cnt++;
        if (!step.idx || cnt === step.idx) out.push(c);
      }
    }
    for (var j = 0; j < cs.length; j++) stack.push(cs[j]);
  }
  return out;
}

// 相对 from 的 XPath 查询；expr 以 // 开头时从 from 子树全局匹配，
// 以 / 开头时只匹配 from 的直接子节点。返回匹配节点数组。
function kzXPath(from, expr) {
  var steps = kzParse(expr);
  if (!steps.length) return [];
  var abs = (expr || '').trim().indexOf('/') === 0 && (expr || '').trim().indexOf('//') !== 0;
  var nodes = abs ? kzChildren(from) : kzStep(from, steps[0]);
  for (var s = 1; s < steps.length; s++) {
    var step = steps[s];
    if (step.text) {
      // text() 必须是最后一段
      var txt = [];
      for (var i = 0; i < nodes.length; i++) {
        var t = '';
        var cs = kzChildren(nodes[i]);
        for (var j = 0; j < cs.length; j++) if (cs[j].x !== undefined) t += cs[j].x;
        txt.push(t.trim());
      }
      return txt;
    }
    var next = [];
    for (var k = 0; k < nodes.length; k++) {
      var found = kzStep(nodes[k], step);
      for (var f = 0; f < found.length; f++) next.push(found[f]);
    }
    nodes = next;
  }
  return nodes;
}

function kzXPath1(from, expr) {
  var r = kzXPath(from, expr);
  return r.length ? r[0] : null;
}

// 节点直接文本
function kzText(node) {
  if (!node) return '';
  if (node.x !== undefined) return node.x.trim();
  var t = '';
  var cs = kzChildren(node);
  for (var i = 0; i < cs.length; i++) if (cs[i].x !== undefined) t += cs[i].x;
  return t.trim();
}

// href 提取
function kzHref(node) {
  if (!node || !node.a) return '';
  return node.a.href || node.a.src || '';
}

// 相对 URL → 绝对
function kzAbs(base, u) {
  u = (u || '').trim();
  if (!u) return '';
  if (u.indexOf('http') === 0) return u;
  if (u.indexOf('//') === 0) return (base.indexOf('https') === 0 ? 'https:' : 'http:') + u;
  if (u.indexOf('/') === 0) {
    var m = /^(https?:\/\/[^\/]+)/.exec(base);
    return m ? m[1] + u : u;
  }
  var slash = base.lastIndexOf('/');
  return base.slice(0, slash + 1) + u;
}

// 通用播放页解析：video/source/embed src → 直链 → iframe 递归一层
function kzParsePage(url, depth, headers) {
  if (depth > 2) return { urls: [], kind: 'video' };
  return fetch(url, { headers: headers }).then(function (r) {
    if (r.status !== 200) return { urls: [], kind: 'video' };
    var html = r.body || '';
    var found = [];
    var m;
    var re1 = /<(?:video|source|embed)[^>]+src=["']([^"']+)["']/gi;
    while ((m = re1.exec(html)) !== null) found.push(m[1]);
    var re2 = /https?:\/\/[^"'\s<>]+?\.(?:m3u8|mp4|flv|webm)(?:\?[^"'\s<>]*)?/gi;
    while ((m = re2.exec(html)) !== null) found.push(m[0]);
    var re3 = /<(?:video|source)[^>]+src=["'](\/[^"']+)["']/gi;
    while ((m = re3.exec(html)) !== null) found.push(kzAbs(url, m[1]));
    if (found.length) {
      var uniq = [];
      for (var i = 0; i < found.length; i++) {
        var u = found[i];
        if (u.indexOf('http') !== 0) u = kzAbs(url, u);
        if (uniq.indexOf(u) < 0) uniq.push(u);
      }
      return { urls: uniq, kind: 'video' };
    }
    var ifre = /<iframe[^>]+src=["']([^"']+)["']/i.exec(html);
    if (ifre) return kzParsePage(kzAbs(url, ifre[1]), depth + 1, headers);
    return { urls: [], kind: 'video' };
  }).catch(function () { return { urls: [], kind: 'video' }; });
}
