# 萌游 MoeGame 更新服务器部署手册（192.168.2.88）

目标：把 Windows 安装包放进更新服务器后，局域网内所有已安装客户端在「设置 → 应用更新」中自动收到新版本；同时 `http://192.168.2.88:8788/` 提供网页下载入口。

## 一、组件说明（本文件夹）

| 文件 | 作用 |
| --- | --- |
| `install-update-server.cmd` | 一键安装入口（右键管理员运行，会自动请求提权） |
| `install.ps1` | 实际安装逻辑：复制到 `C:\MoePlayUpdateServer`、开防火墙、注册开机自启计划任务、生成发布令牌 |
| `server.ps1` | 更新服务本体（PowerShell HttpListener，零外部依赖，端口 8788） |
| `index.html` | 下载页（自动读取最新版本与历史安装包） |
| `uninstall.ps1` | 卸载（保留已发布的安装包数据） |

服务器不需要安装 Node、nginx 或任何软件，Windows 10/11 / Server 2016+ 自带环境即可。

## 二、部署步骤（服务器上做，约 1 分钟）

1. 把整个 `update-server` 文件夹拷贝到 192.168.2.88 那台机器上（U 盘、共享文件夹、远程桌面复制均可）。
2. 右键 `install-update-server.cmd` → **以管理员身份运行**。
3. 弹窗显示安装成功后，记下两行信息：
   - `Publish token`（发布令牌，一串 64 位字符）
   - 服务已监听 `http://<本机IP>:8788`
4. 在本机（开发 PC）浏览器打开 `http://192.168.2.88:8788/`，能看到「尚未发布任何版本」的下载页即部署成功。

> 重复运行 `install-update-server.cmd` 是安全的（幂等更新）；加 `-KeepExistingToken` 参数重装可保留原令牌。
> 换端口：`powershell -File install.ps1 -Port 9000`（同时需把 `tauri.conf.json` 端点里的端口改掉并重新构建客户端）。

## 三、把令牌填回开发 PC（一次性）

编辑 `C:\Users\sgy\.tauri\moeplay-publish-config.json`，把服务器打印的令牌填进去：

```json
{
  "privateKeyPath": "C:/Users/sgy/.tauri/moeplay_updater.key",
  "password": "（已填好）",
  "serverBaseUrl": "http://192.168.2.88:8788",
  "publishToken": "把服务器打印的 Publish token 粘贴到这里"
}
```

## 四、日常发布（以后每次发版只做这一步）

在开发 PC 的项目目录里执行一条命令：

```powershell
npm run release:win -- --notes "这次更新了什么"
```

它会自动：构建安装包 → minisign 签名 → 上传安装包/签名 → 原子替换服务器上的 `latest.json`。

发布完成的标志：

- 终端输出 `✔ vX.Y.Z 已发布`；
- 打开 `http://192.168.2.88:8788/` 看到新版本卡片；
- 各客户端「设置 → 应用更新」提示发现新版本，点「下载并安装」→「立即重启」即完成升级。

已有构建产物时也可单独发布：`npm run publish:update`（默认取 `src-tauri/target/release/bundle/nsis`）。

## 五、故障排查

| 现象 | 处理 |
| --- | --- |
| 客户端提示「更新检查失败」 | 确认服务器开机且服务在跑：服务器上执行 `Start-ScheduledTask -TaskName MoePlayUpdateServer`；查看 `C:\MoePlayUpdateServer\server.log` |
| 安装脚本报 HttpListener 拒绝访问 | 必须以管理员运行 `install-update-server.cmd`（普通双击不行） |
| 发布时 401 invalid publish token | 令牌不一致：服务器 `C:\MoePlayUpdateServer\server.config.json` 里的 `token` 与 PC 配置对齐 |
| 发布时连不上服务器 | 防火墙规则是否被删；`Test-NetConnection 192.168.2.88 -Port 8788` 验证 |
| 端口 8788 被占用 | `install.ps1 -Port 新端口` 重装，并同步修改 `tauri.conf.json` 端点后重新构建客户端 |

## 六、安全边界

- 上传接口（PUT）必须带发布令牌，否则 401；下载接口只读。
- 更新包全部经过 minisign 签名校验，即使局域网流量被篡改，客户端也会拒装。
- 私钥与口令只存在于开发 PC 的 `%USERPROFILE%\.tauri\` 下，**不在仓库、不在服务器**。请务必自行备份该目录（丢失后所有客户端将无法再自动更新，只能重新逐台手动安装）。
