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
# 登录认证 (会自动检测网络，若已在线则直接退出)
# 安全方式: 从 stdin 管道读取密码
echo "YourPassword" | ./i-wzu-auth login -u YourAccount --password-stdin

# 安全方式: 交互式输入密码 (无回显)
./i-wzu-auth login -u YourAccount --password-stdin

# 传统方式 (⚠️ 密码会留在 shell 历史中)
./i-wzu-auth login -u <YourAccount> -p <YourPassword>

# 查看当前在线状态 (流量、时长等)
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
| `--password-stdin` | `-P` | 从标准输入读取密码 (安全) | — |
| `--save` | `-s` | 登录成功后以**明文**保存配置到本地文件 (权限 0600) | — |
| `--ac-id` | `-a` | 网关节点 ID | `2` (温大通常为 2) |
| `--force` | `-f` | 强制执行登录，跳过登录前后的联网检测 | `false` |
| `--dual-stack` | `-d` | 启用双栈认证 (IPv4/IPv6 Dual Stack) | `false` |
| `--check-url` | — | 联网检测 URL (HTTP 204 即为已联网) | `http://connect.rom.miui.com/generate_204` / `SRUN_CHECK_URL` |

### 子命令 (Subcommands)

- **`login`**: 执行认证流程。程序会先检测网络，若不可达则尝试登录。
- **`status`**: 查询状态。无需参数，自动识别当前 IP 的流量和时长信息。
- **`logout`**: 注销下线。无需参数，自动识别当前在线账号并申请注销。

---

## 🔒 安全使用指南

为避免密码泄露，请优先使用以下安全方式：

### 方式 1：`--password-stdin` (推荐)

```bash
# 从文件读取密码 (文件权限建议设为 600)
./i-wzu-auth login -u 账号 --password-stdin < /etc/i-wzu-auth/pass

# 从管道输入
echo "密码" | ./i-wzu-auth login -u 账号 --password-stdin

# 交互式输入 (终端下密码不可见)
./i-wzu-auth login -u 账号 --password-stdin
```

### 方式 2：配置文件 (最便捷)

```bash
# 首次登录时保存配置
echo "密码" | ./i-wzu-auth login -u 账号 --password-stdin --save

# 后续直接登录，无需任何凭证
./i-wzu-auth login
```

配置文件以**明文 JSON** 存储在 `~/.config/i-wzu-auth/config.json`，权限 `600`（仅 owner 可读写）。安全性完全依赖文件权限。

### 不安全的方式 (⚠️ 仅供临时使用)

- **`-p` 参数**：密码会留在 shell 历史 (`.bash_history` 等) 中
- **`SRUN_PASS` 环境变量**：密码可在 `/proc/*/environ` 中被其他用户查看

---

## 💡 自动化建议 (Headless)

由于本工具是 CLI 程序，你可以配合 `crontab` 或 `systemd` 实现断网自动重连：

### 方式 1：使用配置文件
```bash
# 一次性设置 (密码不会留在 shell 历史中)
echo "你的密码" | ./i-wzu-auth login -u 账号 --password-stdin --save

# crontab — 无需任何凭证
* * * * * /path/to/i-wzu-auth login >> /var/log/srun.log 2>&1
```

### 方式 2：使用 --password-stdin + 密码文件
```bash
# 创建受保护的密码文件
echo "你的密码" > /etc/i-wzu-auth/pass
chmod 600 /etc/i-wzu-auth/pass

# crontab
* * * * * /path/to/i-wzu-auth login -u 账号 --password-stdin < /etc/i-wzu-auth/pass >> /var/log/srun.log 2>&1
```

---

## 📜 许可证
MIT License.

---
*Developed with ❤️ for WZU Students.*
