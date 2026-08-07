# TUI 安装服务公网部署指南

本文说明如何把 YunDrone BLE 的 TUI 安装器发布到公网静态服务器，让用户可以通过下面两个入口安装或启动工具：

```bash
bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh)
bash <(curl -fsSL https://install.yundrone.cn/ble-server.sh)
```

这不是目标设备上的 server 安装流程，而是维护者把安装入口、TUI 脚本、gum 工具包、client/server release 包同步到 `install.yundrone.cn` 的发布流程。

## 发布内容

发布脚本会组装一个静态站点目录，然后通过 SSH 上传到公网服务器。

| 路径 | 作用 |
| --- | --- |
| `/ble-wifi-tool.sh` | 用户推荐入口，打开统一 TUI，可选择部署被控端或启动 client。 |
| `/ble-server.sh` | 兼容入口，只进入被控端安装器。 |
| `/yundrone/ble/launcher/installer/latest.json` | 统一 TUI launcher 的版本清单。 |
| `/yundrone/ble/launcher/installer/versions/<VERSION>/` | 统一 TUI launcher 的版本化脚本文件。 |
| `/yundrone/ble-server/installer/latest.json` | 被控端 installer 的版本清单。 |
| `/yundrone/ble-server/installer/versions/<VERSION>/` | 被控端 installer 的版本化脚本文件。 |
| `/yundrone/ble/tools/gum/latest.json` | gum TUI 二进制工具包清单。 |
| `/yundrone/ble-client/releases/latest.json` | client CLI 裸二进制 release 清单。 |
| `/yundrone/ble-server/releases/latest.json` | server release 清单。 |
| `/yundrone/ble-server/releases/<VERSION>/` | server tarball 和版本级 `release.json`。 |

入口脚本本身很小。它们会读取 `latest.json`，下载对应版本的脚本目录，再执行 `main.sh`。

## 远端服务器要求

公网服务器只需要能托管静态文件。

- 域名：`install.yundrone.cn`
- HTTPS：必须可用，用户命令默认使用 `https://`
- 静态根目录：默认 `/var/www/install.yundrone.cn`
- 运维访问：本地机器能通过 SSH 登录，默认 host alias 是 `self-cloudserver`
- 权限：远端用户需要能用 `sudo` 创建目录、解压 tarball、删除临时包

Nginx 只需要把 `install.yundrone.cn` 指向静态根目录。推荐至少支持这些文件类型：`.sh`、`.json`、`.tar.gz`。不要给入口脚本做 HTML fallback，否则 `curl -fsSL` 会下载到网页而不是 shell。

## 本地准备

建议在准备发布前完成：

```bash
scripts/ci/check.sh quality
```

需要的本地工具：

- `bash`
- `python3`
- `ssh` / `scp`
- `tar`
- `curl`
- `docker`，用于构建 Linux amd64/arm64 release 包
- Rust toolchain，版本由 `rust-toolchain.toml` 固定

发布版本来自仓库根目录的 `VERSION`。如果只修安装脚本但仍发布到当前版本号，确认这是有意行为；否则应先更新 `VERSION` 和 `CHANGELOG`，并按 release 流程创建 tag。

## 构建 Release 包

上传脚本会自动发布安装器脚本和 gum 工具包，但 client/server 二进制 tarball 需要先准备到 `dist/`。

构建 Linux server：

```bash
PLATFORMS="linux-amd64 linux-arm64" \
  scripts/release/package-server-release-docker.sh
```

构建 Linux client：

```bash
PLATFORMS="linux-amd64 linux-arm64" \
  scripts/release/package-client-release-docker.sh
```

在 macOS Apple Silicon 上构建 macOS client 裸 CLI：

```bash
PLATFORM=macos-arm64 scripts/release/package-client-release.sh
```

构建完成后应看到类似目录：

```text
dist/ble-server/releases/<VERSION>/yundrone-ble-server-linux-amd64.tar.gz
dist/ble-server/releases/<VERSION>/yundrone-ble-server-linux-arm64.tar.gz
dist/ble-client/releases/<VERSION>/yundrone-ble-client-linux-amd64.tar.gz
dist/ble-client/releases/<VERSION>/yundrone-ble-client-linux-arm64.tar.gz
dist/ble-client/releases/<VERSION>/yundrone-ble-client-macos-arm64.tar.gz
```

上传脚本要求上述五个 tarball 全部存在。缺少任一文件都会立即停止，不会产生缺少 release metadata 的半成品发布。

## 执行上传

默认上传到：

```text
REMOTE=self-cloudserver
REMOTE_ROOT=/var/www/install.yundrone.cn
```

执行：

```bash
scripts/release/upload-ble-distribution-manual.sh
```

常用环境变量：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `VERSION` | 读取 `VERSION` 文件 | 要发布的版本号。 |
| `YUNDRONE_INSTALL_REMOTE` | `self-cloudserver` | SSH 目标。 |
| `YUNDRONE_INSTALL_REMOTE_ROOT` | `/var/www/install.yundrone.cn` | 远端静态根目录。 |
| `YUNDRONE_INSTALL_REMOTE_SUDO_PASSWORD` | 空 | 非交互 sudo 密码。能用 SSH TTY 或免密 sudo 时不要设置。 |
| `DIST_DIR` | `dist/install-site` | 本地临时静态站点目录。 |
| `GUM_TOOLS_DIR` | `dist/gum-tools` | gum 工具包缓存目录。 |
| `CLIENT_RELEASE_DIR` | `dist/ble-client/releases/<VERSION>` | client tarball 来源目录。 |
| `SERVER_RELEASE_DIR` | `dist/ble-server/releases/<VERSION>` | server tarball 来源目录。 |

上传脚本会：

1. 清空本地 `dist/install-site`。
2. 复制统一 launcher 和 server installer 到版本化目录。
3. 生成 installer manifest。
4. 准备 gum 工具包。如果本地没有 `dist/gum-tools/latest.json`，会从 Charmbracelet GitHub release 下载并重新打包。
5. 校验并复制五个 client/server tarball，生成 `latest.json`。
6. 在远端 `/tmp` 创建当前站点备份。
7. 把整个静态站点打成 tarball，通过 `scp` 上传并覆盖解压。
8. 验证两个入口脚本和四份 live manifest 的版本。
9. 下载每个 manifest 声明的文件，逐项核对 SHA256。

## 验证

发布后先在本地验证入口脚本可下载：

```bash
curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh | head
curl -fsSL https://install.yundrone.cn/ble-server.sh | head
```

验证 metadata：

```bash
curl -fsSL https://install.yundrone.cn/yundrone/ble/launcher/installer/latest.json
curl -fsSL https://install.yundrone.cn/yundrone/ble-server/installer/latest.json
curl -fsSL https://install.yundrone.cn/yundrone/ble/tools/gum/latest.json
curl -fsSL https://install.yundrone.cn/yundrone/ble-client/releases/latest.json
curl -fsSL https://install.yundrone.cn/yundrone/ble-server/releases/latest.json
```

建议做一次非破坏性 smoke test：

```bash
bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh) -- doctor
bash <(curl -fsSL https://install.yundrone.cn/ble-server.sh) -- doctor
```

如果是在没有蓝牙适配器或不支持平台的机器上测试，doctor 可能报告“不支持”或“未发现蓝牙适配器”，这是业务环境问题；关键是入口脚本、manifest 下载和 TUI 引擎下载链路不能失败。

## 回滚

当前上传方式是把新静态站点覆盖解压到远端目录，没有自动保留远端快照。正式发布前建议在远端先做备份：

```bash
ssh self-cloudserver \
  "sudo tar -C /var/www/install.yundrone.cn -czf /tmp/install-yundrone-cn-backup-\$(date +%Y%m%d-%H%M%S).tar.gz ."
```

如果需要回滚：

```bash
scp /path/to/backup.tar.gz self-cloudserver:/tmp/install-yundrone-cn-rollback.tar.gz
ssh self-cloudserver \
  "sudo rm -rf /var/www/install.yundrone.cn/* && sudo tar -xzf /tmp/install-yundrone-cn-rollback.tar.gz -C /var/www/install.yundrone.cn"
```

也可以用旧版本重新生成 `dist/install-site` 并再次运行上传脚本。注意：如果入口脚本和 release metadata 指向旧版本，但远端缺少旧版本 tarball，用户安装仍会失败。

## 常见问题

### 只改了安装器脚本，需要重新构建 server/client 吗？

需要。上传脚本把完整 client/server 资产视为正式发布的硬性条件，缺少当前版本任一 tarball 都会退出。

### `gum latest.json` 是哪里来的？

`scripts/release/package-gum-tools.py` 会下载 Charmbracelet 官方 gum release，提取 `gum` 二进制，再按 YunDrone 的平台命名重新打包到 `dist/gum-tools`。

### 可以换域名或测试环境吗？

可以，但目前脚本里的 manifest base URL 默认写死为 `https://install.yundrone.cn/...`。如果要长期支持 staging 域名，应先给上传脚本增加可配置 base URL，而不是只改远端目录。

### 为什么入口脚本要分 `ble-wifi-tool.sh` 和 `ble-server.sh`？

`ble-wifi-tool.sh` 是推荐的新入口，既能启动 client，也能进入 server 部署。`ble-server.sh` 是兼容入口，只服务被控端安装和维护，方便老文档、老用户和自动化脚本继续使用。

### 部署 Web Client 是否也在这里？

不在这里。Web Client 是 `web-client/dist/` 的纯静态网页，当前文档见 `docs/WEB_CLIENT_ZH.md`。TUI 安装服务部署的是 shell 入口、installer manifest、gum 工具包和 Rust client/server tarball。
