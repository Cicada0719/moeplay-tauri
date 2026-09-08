# 独立下载站

服务器准备 Docker 与 Compose，并为 cloudflared 所在的 Docker 网络设置 `MOEPLAY_TUNNEL_NETWORK`。设置 `MOEPLAY_RELEASE_ROOT` 为发布目录的绝对路径；这些值写入服务器本地 `.env`，不要提交。

```text
发布目录/
  downloads/0.23.0/   # 与 GitHub 完全一致的产物
  sites/0.23.0/       # 网页、截图、release-manifest.json、latest.json
  current -> sites/0.23.0
  incoming/0.23.0/    # 上传暂存区
```

1. 本地跑完测试、构建、安装验收，合并 PR 后将同批文件上传 GitHub。
2. SFTP 上传 `site/` 和产物 `downloads/` 到 `incoming/<版本>/`。产物目录包括签名、latest.json 与统一发布清单。
3. 在服务器执行 `python3 activate.py "$MOEPLAY_RELEASE_ROOT" 0.23.0`，校验大小与 SHA-256 后原子切换 `current`。已存在版本会拒绝覆盖。
4. 在包含 compose.yaml、nginx.conf 与本地 .env 的目录执行 `docker compose up -d`。局域网入口是服务器 `:8788`。
5. Cloudflare Tunnel 添加一个公开主机名，指向同一 Docker 网络内的 `http://moeplay-downloads:80`。保留其他路由与末尾 fallback，不使用 Tunnel 运行令牌操作 DNS。
6. 公网与局域网分别下载完整附件，比较 SHA-256，并检查 Range 请求返回 206、清单 Cache-Control 为 no-store、上传请求被拒绝。

回滚时把新的临时符号链接指向 `deployment-history.jsonl` 中已存在的上个站点目录，再通过 `os.replace` 或 `mv -T` 原子替换 `current`。旧版本目录与旧安装包一直保留。

Nginx 只读挂载发布目录，没有上传 API；只开放网页与下载路径。更新清单中的安装包 URL 必须为 HTTPS，并保留有效 Minisign 签名。
