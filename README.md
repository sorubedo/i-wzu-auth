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
| `--ac-id` | `-a` | 网关节点 ID | `2` (温大通常为 2) |
| `--force` | `-f` | 强制执行登录 (跳过在线检测) | `false` |
| `--dual-stack` | `-d` | 启用双栈认证 (IPv4/IPv6 Dual Stack) | `false` |

### 子命令 (Subcommands)

- **`login`**: 执行认证流程。程序会先检测网络，若不可达则尝试登录。
- **`status`**: 查询状态。无需参数，自动识别当前 IP 的流量和时长信息。
- **`logout`**: 注销下线。无需参数，自动识别当前在线账号并申请注销。

---

## 💡 自动化建议 (Headless)

由于本工具是 CLI 程序，你可以配合 `crontab` 或 `systemd` 实现断网自动重连：
```bash
# crontab 示例：每分钟检查一次网络，如果不通则尝试登录
* * * * * /path/to/i-wzu-auth login -u 账号 -p 密码 >> /var/log/srun.log 2>&1
```

---

## 📜 许可证
MIT License.

---
*Developed with ❤️ for WZU Students.*
