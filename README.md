# I-WZU-AUTH: Headless Srun CLI Client 🚀

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![WZU](https://img.shields.io/badge/adapter-温州大学-blue.svg)](http://www.wzu.edu.cn/)

这是一个专为**温州大学 (WZU)** 适配的深澜 (Srun) 校园网认证 CLI 工具。

**主要针对无头 (Headless) 环境（如 Linux 服务器、路由器、开机脚本等）设计，提供轻量、快速的一键认证方案。**

---

## ✨ 功能特性

- 🖥 **无头环境**：专为命令行设计，无 GUI 依赖，非常适合在服务器、软路由或脚本中调用。
- 🛠 **温大适配**：默认针对温大网关配置，自动获取内网 IP 和网关节点。
- 🔒 **协议支持**：完整实现 Srun 认证所需的 XXTEA、Custom Base64 及 HMAC-MD5 加密。
- 📊 **状态监测**：一键查看在线时长、已用流量、本机 IP 等详细信息。
- 📤 **注销下线**：自动识别当前在线账号并执行注销，无需手动输入账号。
- 🌐 **网卡绑定**：支持强制绑定物理网卡，绕过 TUN 代理（Clash/V2Ray/sing-box）。
- 🎯 **代认证**：支持为指定 IP 地址发起认证/注销，适配路由器旁路场景。
- 🌈 **友好交互**：全中文彩色输出，提供明确的错误排查建议。

---

## 🚀 快速上手

### 1. 编译项目
使用 Cargo 进行发布模式编译：
```bash
cargo build --release
```
编译完成后，可执行文件位于 `target/release/i-wzu-auth`。

### 2. 常用操作
```bash
# 登录认证（交互式输入密码，终端下无回显，不留在 shell 历史）
./i-wzu-auth login -u YourAccount --password-stdin

# 登录认证（传统方式，⚠️ 密码会留在 shell 历史中）
./i-wzu-auth login -u <YourAccount> -p <YourPassword>

# 查看当前在线状态
./i-wzu-auth status

# 注销当前登录
./i-wzu-auth logout
```

---

## 📖 命令行参数详解

### 全局参数 (Global Arguments)
这些参数支持通过命令行指定，或通过环境变量设置：

| 参数 | 短指令 | 说明 | 默认值 / 环境变量 |
| :--- | :--- | :--- | :--- |
| `--url` | `-U` | Srun 认证网关地址 | `http://192.168.16.66` / `SRUN_URL` |
| `--username` | `-u` | 校园网账号 | 无 / `SRUN_USER` |
| `--password` | `-p` | 校园网密码 | 无 / `SRUN_PASS` |
| `--password-stdin` | `-P` | 交互式输入密码（无回显，不留在 shell 历史） | — |
| `--save` | `-s` | 登录成功后以**明文**保存配置到本地文件（权限 0600） | — |
| `--ac-id` | `-a` | 网关节点 ID | `2` (温大通常为 2) |
| `--force` | `-f` | 强制执行登录，跳过登录前后的联网检测 | `false` |
| `--dual-stack` | `-d` | 启用双栈认证 (IPv4/IPv6 Dual Stack) | `false` |
| `--check-url` | — | 联网检测 URL (HTTP 204 即为已联网) | `http://connect.rom.miui.com/generate_204` / `SRUN_CHECK_URL` |
| `--interface` | `-i` | 强制绑定到指定网卡发送请求，绕过 TUN 代理 | 无 / `SRUN_INTERFACE` |
| `--ip` | — | 为指定 IP 地址发起认证/注销/查询（代认证场景） | 无 / `SRUN_IP` |

### 子命令 (Subcommands)

- **`login`**: 执行认证流程。程序会先检测网络，若不可达则尝试登录。
- **`status`**: 查询状态。无需参数，自动识别当前 IP 的流量和时长信息。
- **`logout`**: 注销下线。无需参数，自动识别当前在线账号并申请注销。

---

## 🔀 特殊场景

### 绕过 TUN 代理

当系统运行 Clash、V2Ray、sing-box 等 TUN 模式代理时，校园网内网地址无法通过代理访问，导致认证失败（`no_response_data_error`）。使用 `--interface` 强制绑定物理网卡：

```bash
# 绑定到 eth0 网卡
./i-wzu-auth login -u 账号 -i eth0 --password-stdin

# 绑定到 wlan0 无线网卡
./i-wzu-auth status -i wlan0
```

### 代认证 / 旁路认证

用一台设备为另一台设备（如同网段下的 PC、手机）发起认证：

```bash
# 为 192.168.1.100 登录
./i-wzu-auth login -u 对方账号 --ip 192.168.1.100 --password-stdin

# 注销指定 IP 的设备
./i-wzu-auth logout --ip 192.168.1.100

# 查询指定 IP 的在线状态
./i-wzu-auth status --ip 192.168.1.100
```

指定 `--ip` 后自动跳过联网检测，直接向目标 IP 发送认证请求。

---

## 🔒 安全使用指南

### 方式 1：`--password-stdin`（交互式输入）

```bash
./i-wzu-auth login -u 账号 --password-stdin
```
终端下会弹出 `Password:` 提示，输入时无回显，不留在 shell 历史。

### 方式 2：配置文件（自动化场景最便捷）

```bash
# 首次登录时保存配置
./i-wzu-auth login -u 账号 --password-stdin --save

# 后续直接登录，无需任何凭证
./i-wzu-auth login
```

配置文件以**明文 JSON** 存储在 `~/.config/i-wzu-auth/config.json`，权限 `600`（仅 owner 可读写）。

### 不安全的方式

- **`-p` 参数**：密码会留在 shell 历史 (`.bash_history` 等)
- **`SRUN_PASS` 环境变量**：密码可在 `/proc/*/environ` 中被其他用户查看

---

## 💡 自动化建议 (Headless)

配合 `crontab` 或 `systemd` 实现断网自动重连：

```bash
# 一次性设置
./i-wzu-auth login -u 账号 --password-stdin --save

# crontab — 无需任何凭证
* * * * * /path/to/i-wzu-auth login >> /var/log/srun.log 2>&1
```

---

## 📜 许可证
MIT License.

---
*Developed with ❤️ for WZU Students.*
