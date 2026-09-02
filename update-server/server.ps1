# MoePlay 局域网更新服务（萌游 MoeGame 安装包自动发布/下载/更新检查）
# 零依赖：仅使用 Windows 内置 PowerShell 5.1 + HttpListener。
# 目录结构（与 server.config.json 同级）：
#   www/latest.json          —— Tauri 更新器清单（由 PC 端发布脚本原子写入）
#   www/installers/<file>    —— NSIS 安装包与 .sig 签名
#   index.html               —— 下载页（GET / 返回）
# 路由：
#   GET  /                       下载页
#   GET  /latest.json            更新清单（no-store，保证客户端总拿最新）
#   GET  /installers/<name>      安装包/签名下载（attachment）
#   GET  /api/state              JSON：当前清单 + 全部历史文件
#   PUT  /api/upload/installer?name=<n>   上传安装包（需 X-Publish-Token）
#   PUT  /api/upload/signature?name=<n>   上传 .sig（需 X-Publish-Token）
#   PUT  /api/upload/manifest             原子替换 latest.json（需 X-Publish-Token）
param()
$ErrorActionPreference = "Stop"
$BaseDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Config = Get-Content -Raw -Encoding UTF8 (Join-Path $BaseDir "server.config.json") | ConvertFrom-Json
$Port = [int]$Config.port
$Token = $Config.token
$WwwRoot = Join-Path $BaseDir "www"
$InstallersDir = Join-Path $WwwRoot "installers"
New-Item -ItemType Directory -Force -Path $InstallersDir | Out-Null
$LogFile = Join-Path $BaseDir "server.log"

function Write-Log([string]$msg) {
    $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $msg
    Write-Host $line
    Add-Content -Path $LogFile -Value $line -Encoding UTF8
}

function Send-Text($Ctx, [int]$Code, [string]$Body, [string]$ContentType) {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Body)
    $Ctx.Response.StatusCode = $Code
    $Ctx.Response.ContentType = $ContentType
    $Ctx.Response.Headers.Add("Cache-Control", "no-store")
    $Ctx.Response.ContentLength64 = $bytes.Length
    $Ctx.Response.OutputStream.Write($bytes, 0, $bytes.Length)
}

function Send-File($Ctx, [string]$Path, [bool]$AsAttachment) {
    if (-not (Test-Path -LiteralPath $Path)) { Send-Text $Ctx 404 "not found" "text/plain; charset=utf-8"; return }
    $info = Get-Item -LiteralPath $Path
    $Ctx.Response.StatusCode = 200
    if ($info.Name -match '\.json$') { $Ctx.Response.ContentType = "application/json" }
    elseif ($info.Name -match '\.html$') { $Ctx.Response.ContentType = "text/html; charset=utf-8" }
    elseif ($info.Name -match '\.sig$') { $Ctx.Response.ContentType = "text/plain" }
    else { $Ctx.Response.ContentType = "application/octet-stream" }
    if ($AsAttachment) {
        $encoded = [Uri]::EscapeDataString($info.Name)
        $Ctx.Response.AddHeader("Content-Disposition", "attachment; filename=`"$encoded`"; filename*=UTF-8''$encoded")
    }
    $Ctx.Response.Headers.Add("Cache-Control", "no-store")
    $Ctx.Response.ContentLength64 = $info.Length
    $fs = [System.IO.File]::OpenRead($Path)
    try { $fs.CopyTo($Ctx.Response.OutputStream) } finally { $fs.Close() }
}

function Read-BodyBytes($Ctx) {
    $ms = New-Object System.IO.MemoryStream
    $Ctx.Request.InputStream.CopyTo($ms)
    return $ms.ToArray()
}

function Test-Token($Ctx) {
    $given = $Ctx.Request.Headers["X-Publish-Token"]
    return ($null -ne $given) -and ($given -ceq $Token)
}

$Listener = New-Object System.Net.HttpListener
$Listener.Prefixes.Add("http://+:$Port/")
try {
    $Listener.Start()
} catch {
    Write-Log "全网卡监听失败，降级为仅本机回环（局域网客户端将无法访问）：$($_.Exception.Message)"
    $Listener = New-Object System.Net.HttpListener
    $Listener.Prefixes.Add("http://127.0.0.1:$Port/")
    try {
        $Listener.Start()
    } catch {
        Write-Log "HttpListener 启动失败（端口 $Port 可能被占用）：$($_.Exception.Message)"
        exit 1
    }
}
Write-Log "MoePlay 更新服务已启动，端口 $Port，根目录 $WwwRoot"

while ($Listener.IsListening) {
    try { $Ctx = $Listener.GetContext() } catch { break }
    try {
        $Req = $Ctx.Request
        $Res = $Ctx.Response
        $resPath = [Uri]::UnescapeDataString($Req.Url.AbsolutePath)
        $method = $Req.HttpMethod
        if ($method -eq "OPTIONS" -or $method -eq "HEAD") { $Res.StatusCode = 200; $Res.Close(); continue }

        if ($method -eq "GET") {
            switch -Regex ($resPath) {
                "^/$" { Send-File $Ctx (Join-Path $BaseDir "index.html") $false; continue }
                "^/latest\.json$" { Send-File $Ctx (Join-Path $WwwRoot "latest.json") $false; continue }
                "^/api/state$" {
                    $latest = $null
                    $latestPath = Join-Path $WwwRoot "latest.json"
                    if (Test-Path $latestPath) { $latest = Get-Content -Raw -Encoding UTF8 $latestPath | ConvertFrom-Json }
                    $files = @()
                    if (Test-Path $InstallersDir) {
                        $files = Get-ChildItem $InstallersDir | Where-Object { -not $_.PSIsContainer } | Sort-Object -Descending LastWriteTime | ForEach-Object {
                            @{ name = $_.Name; size = $_.Length; mtime = $_.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss") }
                        }
                    }
                    Send-Text $Ctx 200 (@{ latest = $latest; files = $files } | ConvertTo-Json -Depth 6) "application/json"
                    continue
                }
                "^/installers/[A-Za-z0-9._\-]+$" {
                    $name = Split-Path -Leaf $resPath
                    Send-File $Ctx (Join-Path $InstallersDir $name) ($name -notmatch '\.sig$')
                    continue
                }
                default { Send-Text $Ctx 404 "not found" "text/plain; charset=utf-8" }
            }
        }
        elseif ($method -eq "PUT") {
            if (-not (Test-Token $Ctx)) { Send-Text $Ctx 401 "invalid publish token" "text/plain; charset=utf-8"; continue }
            switch ($resPath) {
                "/api/upload/installer" {
                    $name = $Req.QueryString["name"]
                    if (-not $name -or $name -notmatch '^[A-Za-z0-9._\-]+\.exe$') { Send-Text $Ctx 400 "bad name" "text/plain; charset=utf-8"; continue }
                    $body = Read-BodyBytes $Ctx
                    $target = Join-Path $InstallersDir $name
                    $tmp = "$target.tmp"
                    [System.IO.File]::WriteAllBytes($tmp, $body)
                    Move-Item -Force -LiteralPath $tmp -Destination $target
                    Send-Text $Ctx 200 "ok" "text/plain; charset=utf-8"
                    Write-Log "上传安装包 $name（$([math]::Round($body.Length/1MB,1)) MB）"
                    continue
                }
                "/api/upload/signature" {
                    $name = $Req.QueryString["name"]
                    if (-not $name -or $name -notmatch '^[A-Za-z0-9._\-]+\.exe\.sig$') { Send-Text $Ctx 400 "bad name" "text/plain; charset=utf-8"; continue }
                    $body = Read-BodyBytes $Ctx
                    $target = Join-Path $InstallersDir $name
                    [System.IO.File]::WriteAllBytes("$target.tmp", $body)
                    Move-Item -Force -LiteralPath "$target.tmp" -Destination $target
                    Send-Text $Ctx 200 "ok" "text/plain; charset=utf-8"
                    Write-Log "上传签名 $name"
                    continue
                }
                "/api/upload/manifest" {
                    $body = Read-BodyBytes $Ctx
                    try { $json = [System.Text.Encoding]::UTF8.GetString($body) | ConvertFrom-Json } catch { Send-Text $Ctx 400 "invalid json" "text/plain; charset=utf-8"; continue }
                    if (-not $json.version -or -not $json.platforms) { Send-Text $Ctx 400 "manifest missing version/platforms" "text/plain; charset=utf-8"; continue }
                    $target = Join-Path $WwwRoot "latest.json"
                    $pretty = $json | ConvertTo-Json -Depth 8
                    [System.IO.File]::WriteAllBytes("$target.tmp", [System.Text.Encoding]::UTF8.GetBytes($pretty))
                    Move-Item -Force -LiteralPath "$target.tmp" -Destination $target
                    Send-Text $Ctx 200 "ok" "text/plain; charset=utf-8"
                    Write-Log "发布新清单 latest.json → v$($json.version)"
                    continue
                }
                default { Send-Text $Ctx 404 "not found" "text/plain; charset=utf-8" }
            }
        }
        else { Send-Text $Ctx 405 "method not allowed" "text/plain; charset=utf-8" }
        $Ctx.Response.Close()
    } catch {
        Write-Log "请求处理异常：$($_.Exception.Message)"
        try { $Ctx.Response.StatusCode = 500; $Ctx.Response.Close() } catch {}
    }
}
