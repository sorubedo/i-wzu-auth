mod api;
mod config;
mod crypto;

use api::SrunClient;
use clap::{Parser, Subcommand};
use colored::*;
use std::io::{self, Write};

#[derive(Parser, Debug)]
#[command(
    name = "i-wzu-auth",
    author = "sorubedo",
    version = env!("CARGO_PKG_VERSION"),
    about = "🚀 温州大学 Srun 校园网无头 (Headless) 认证工具 (Rust)",
    long_about = None
)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Srun 认证网关地址
    #[arg(
        short = 'U',
        long,
        env = "SRUN_URL",
        global = true,
        default_value = "http://192.168.16.66"
    )]
    url: String,

    /// 联网检测 URL (HTTP 204 即为已联网)，国内用户可设为 connect.rom.miui.com/generate_204
    #[arg(
        long,
        env = "SRUN_CHECK_URL",
        global = true,
        default_value = "http://connect.rom.miui.com/generate_204"
    )]
    check_url: String,

    /// 校园网账号 (登录时必填)
    #[arg(short = 'u', long, env = "SRUN_USER", global = true)]
    username: Option<String>,

    /// 校园网密码 (登录时必填)
    #[arg(short = 'p', long, env = "SRUN_PASS", global = true)]
    password: Option<String>,

    /// 从标准输入读取密码 (替代 -p，避免密码出现在命令行历史中)
    #[arg(short = 'P', long, global = true, conflicts_with = "password")]
    password_stdin: bool,

    /// 登录成功后以明文保存配置到本地 (~/.config/i-wzu-auth/config.json, 权限 0600)
    #[arg(short = 's', long, global = true)]
    save: bool,

    /// 网关节点 ID (AC ID)，温州大学通常为 2
    #[arg(short = 'a', long, default_value = "2", global = true)]
    ac_id: String,

    /// 强制执行登录，跳过登录前后的联网检测
    #[arg(short, long, global = true)]
    force: bool,

    /// 启用双栈认证 (IPv4/IPv6 Dual Stack)
    #[arg(short = 'd', long, global = true)]
    dual_stack: bool,

    /// 强制绑定到指定网卡发送请求（如 eth0、wlan0），用于绕过 TUN 代理
    /// 需要 root 权限或 CAP_NET_RAW capability
    #[arg(
        short = 'i',
        long,
        global = true,
        value_name = "IFACE",
        env = "SRUN_INTERFACE"
    )]
    interface: Option<String>,

    /// 为指定 IP 地址发起认证（用于代认证/旁路认证场景）
    /// 指定后将跳过联网检测，直接向目标 IP 发送认证请求
    #[arg(long, global = true, value_name = "IP", env = "SRUN_IP")]
    ip: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 执行登录认证 (默认行为)
    Login,
    /// 注销当前登录 (自动识别在线账号)
    Logout,
    /// 查看当前在线状态和详细流量信息
    Status,
}

/// 从标准输入读取密码
///
/// 支持两种模式：
/// - TTY 模式：使用 rpassword 实现无回显交互输入
/// - 管道模式：从重定向/管道读取一行
fn read_password_from_stdin() -> String {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        // TTY 模式：无回显输入
        rpassword::prompt_password("Password: ").unwrap_or_else(|e| {
            eprintln!("{} 读取密码失败: {}", "❌".red(), e);
            std::process::exit(1);
        })
    } else {
        // 管道/重定向模式
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).unwrap_or_else(|e| {
            eprintln!("{} 从标准输入读取密码失败: {}", "❌".red(), e);
            std::process::exit(1);
        });
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() {
            eprintln!("{} 标准输入中未提供密码", "❌".red());
            std::process::exit(1);
        }
        trimmed
    }
}

fn main() {
    let args = Args::parse();

    println!("{}", "\n==========================================".cyan());
    println!(
        "   {} v{}",
        "🚀 I-WZU-AUTH SRUN CLIENT".bold().bright_yellow(),
        env!("CARGO_PKG_VERSION").dimmed()
    );
    println!("{}", "==========================================\n".cyan());

    let url = args.url;
    let check_url = args.check_url;
    let ac_id = args.ac_id;
    let dual_stack = args.dual_stack;
    let interface = args.interface.as_deref();
    let target_ip = args.ip.as_deref();

    match args.command.unwrap_or(Commands::Login) {
        Commands::Login => {
            // 优雅地检查账号密码
            let username = match args.username {
                Some(u) => u,
                None => {
                    // 尝试从配置文件读取用户名
                    if config::config_exists() {
                        match config::load_config() {
                            Ok((saved_user, _)) => saved_user,
                            Err(e) => {
                                eprintln!(
                                    "{} {} {}\n",
                                    "❌".red(),
                                    "读取配置文件失败:".red().bold(),
                                    e.white()
                                );
                                std::process::exit(1);
                            }
                        }
                    } else {
                        eprintln!(
                            "{} {} {}\n",
                            "❌".red(),
                            "参数错误:".red().bold(),
                            "登录需要指定账号。".white()
                        );
                        eprintln!(
                            "{} 请使用 {} 参数或设置 {} 环境变量。\n",
                            "  ".dimmed(),
                            "-u".cyan(),
                            "SRUN_USER".cyan()
                        );
                        std::process::exit(1);
                    }
                }
            };
            let password = if args.password_stdin {
                // 如果同时通过环境变量 SRUN_PASS 提供了密码，发出警告
                if args.password.is_some() {
                    eprintln!(
                        "{} 同时指定了 --password-stdin 和 -p/SRUN_PASS，将使用 --password-stdin。",
                        "⚠️".yellow()
                    );
                }
                read_password_from_stdin()
            } else if let Some(p) = args.password {
                p
            } else if config::config_exists() {
                match config::load_config() {
                    Ok((_, saved_pass)) => saved_pass,
                    Err(e) => {
                        eprintln!(
                            "{} {} {}\n",
                            "❌".red(),
                            "读取配置文件失败:".red().bold(),
                            e.white()
                        );
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!(
                    "{} {} {}\n",
                    "❌".red(),
                    "参数错误:".red().bold(),
                    "登录需要指定密码。".white()
                );
                eprintln!(
                    "{} 请使用 {} 参数、{} 参数、{} 环境变量或 {} 保存配置。\n",
                    "  ".dimmed(),
                    "-p".cyan(),
                    "--password-stdin".cyan(),
                    "SRUN_PASS".cyan(),
                    "--save".cyan()
                );
                std::process::exit(1);
            };

            if let Some(ip) = target_ip {
                println!(
                    "{} {} {} {}",
                    "🎯".blue(),
                    "目标 IP:".white(),
                    ip.cyan().bold(),
                    "（跳过联网检测）".dimmed()
                );
            } else if !args.force {
                print!("{} {} ", "🔍".blue(), "正在检测网络连通性...".white());
                if SrunClient::check_online(&check_url, interface) {
                    println!("{}", "已联网 (Online)".green().bold());
                    println!("{}\n", "✨ 您已处于在线状态，无需重复登录。".bright_green());
                    return;
                }
                println!("{}", "未联网 (Offline)".yellow());
            }

            let client = SrunClient::new(
                &url, &username, &password, &ac_id, dual_stack, interface, target_ip,
            );
            println!(
                "{} {} {}",
                "🔑".blue(),
                "正在为用户".white(),
                username.cyan().bold()
            );
            if dual_stack {
                println!("{} {}", "🌐".blue(), "认证模式: IPv4/IPv6 双栈".white());
            }
            println!("{} {}", "📡".blue(), "发起认证请求...".white());

            match client.login() {
                Ok(resp) => {
                    if resp.res == "ok" {
                        println!(
                            "{} {}",
                            "✅".green(),
                            "登录成功！服务器响应: OK".green().bold()
                        );

                        // 保存配置到本地文件
                        if args.save {
                            match config::save_config(&username, &password) {
                                Ok(()) => println!(
                                    "{} 配置已保存到 {}",
                                    "🔐".green(),
                                    config::config_path().display().to_string().dimmed()
                                ),
                                Err(e) => eprintln!("{} 保存配置失败: {}", "⚠️".yellow(), e),
                            }
                        }
                    } else {
                        let error_msg = resp
                            .error_msg
                            .clone()
                            .unwrap_or_else(|| "未知错误".to_string());
                        let error_code = resp.error.clone().unwrap_or_else(|| "N/A".to_string());

                        let is_no_response = resp.res == "no_response_data_error"
                            || error_code == "no_response_data_error"
                            || error_msg.contains("no_response_data_error");

                        eprintln!(
                            "\n{} {} {}",
                            "❌".red(),
                            "登录失败:".red().bold(),
                            if is_no_response {
                                "no_response_data_error".bright_red()
                            } else {
                                error_msg.bright_red()
                            }
                        );

                        if is_no_response {
                            println!(
                                "\n{} {} {}",
                                "💡".yellow(),
                                "温馨提示:".yellow().bold(),
                                "检测到 `no_response_data_error`。".white()
                            );
                            println!(
                                "   {} {} {}",
                                "➤".cyan(),
                                "您可能正在使用虚拟网卡代理".cyan().bold(),
                                "(如 Clash/V2Ray/sing-box TUN 模式)？".cyan()
                            );
                            println!("   {} {}", "➤".cyan(), "建议尝试以下方法之一：".cyan());
                            println!(
                                "     1. {}：{}",
                                "关闭代理后重试".cyan().bold(),
                                "临时关闭 TUN 代理再执行认证".dimmed()
                            );
                            println!(
                                "     2. {}：{}",
                                "使用网卡绑定参数".cyan().bold(),
                                format!("{} -i <物理网卡名> (如 eth0, wlan0)", "i-wzu-auth")
                                    .dimmed()
                            );
                            println!();
                        }
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "\n{} {} {}\n",
                        "🚨".red(),
                        "发生严重错误:".red().bold(),
                        e.bright_red()
                    );
                    std::process::exit(1);
                }
            }

            if target_ip.is_some() {
                println!(
                    "{} {}",
                    "⏩".yellow(),
                    "已跳过二次联网验证（指定了目标 IP）".dimmed()
                );
                println!(
                    "\n{}\n",
                    "🎉 认证流程全部完成，祝您用网愉快！".bright_green().bold()
                );
            } else if !args.force {
                print!("{} {} ", "🔬".blue(), "正在进行二次联网验证".white());
                for i in (1..=3).rev() {
                    print!("{} ", i.to_string().cyan());
                    io::stdout().flush().unwrap();
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }

                if SrunClient::check_online(&check_url, interface) {
                    println!("{}", "验证通过 (Success)".green().bold());
                    println!(
                        "\n{}\n",
                        "🎉 认证流程全部完成，祝您用网愉快！".bright_green().bold()
                    );
                } else {
                    println!("{}", "验证失败 (Failed)".red().bold());
                    std::process::exit(1);
                }
            } else {
                println!("{} 已跳过二次联网验证", "⏩".yellow());
                println!(
                    "\n{}\n",
                    "🎉 认证流程全部完成，祝您用网愉快！".bright_green().bold()
                );
            }
        }
        Commands::Logout => {
            let client = SrunClient::new(&url, "", "", &ac_id, dual_stack, interface, target_ip);
            println!("{} {}", "📊".blue(), "正在识别当前在线账号...".white());

            let logout_target = target_ip.unwrap_or("");
            match client.check_info(logout_target) {
                Ok(info) => {
                    if let Some(online_user) = info["user_name"].as_str() {
                        println!(
                            "{} {} {}",
                            "👤".blue(),
                            "检测到在线用户:".white(),
                            online_user.cyan().bold()
                        );
                        let logout_client = SrunClient::new(
                            &url,
                            online_user,
                            "",
                            &ac_id,
                            dual_stack,
                            interface,
                            target_ip,
                        );
                        println!("{} {}", "📡".blue(), "正在发起注销请求...".white());
                        match logout_client.logout() {
                            Ok(_) => {
                                println!(
                                    "{} {}",
                                    "✅".green(),
                                    "注销成功！计费已停止。".green().bold()
                                );
                                // 注销后延迟验证
                                print!("{} {} ", "🔍".blue(), "正在验证断网状态".white());
                                for i in (1..=3).rev() {
                                    print!("{} ", i.to_string().cyan());
                                    io::stdout().flush().unwrap();
                                    std::thread::sleep(std::time::Duration::from_secs(1));
                                }

                                if SrunClient::check_online(&check_url, interface) {
                                    println!("{}", "仍然在线 (Still Online)".yellow());
                                    println!(
                                        "{} {}\n",
                                        "💡".yellow(),
                                        "网关放行可能存在延迟，请等待物理连接自动断开。".dimmed()
                                    );
                                } else {
                                    println!("{}", "已断开 (Disconnected)".green());
                                }
                            }
                            Err(e) => eprintln!("❌ 注销失败: {}", e),
                        }
                    } else {
                        println!(
                            "{} {}",
                            "ℹ️".yellow(),
                            "您当前似乎并不在线，无需注销。".yellow()
                        );
                    }
                }
                Err(e) => eprintln!("❌ 无法获取在线信息: {}", e),
            }
        }
        Commands::Status => {
            let client = SrunClient::new(&url, "", "", &ac_id, dual_stack, interface, target_ip);
            if let Some(ip) = target_ip {
                println!(
                    "{} {} {} {}",
                    "🎯".blue(),
                    "目标 IP:".white(),
                    ip.cyan().bold(),
                    "（查询指定设备）".dimmed()
                );
            }
            println!("{} {}", "📊".blue(), "正在获取当前在线状态...".white());
            let status_target = target_ip.unwrap_or("");
            match client.check_info(status_target) {
                Ok(info) => {
                    if info["error"].as_str() == Some("not_online_error") {
                        println!("{} {}", "ℹ️".yellow(), "状态: 当前未在线".yellow().bold());
                    } else {
                        println!("{} {}", "🟢".green(), "状态: 已在线".green().bold());

                        println!("\n{}", "--- 👤 账户信息 ---".dimmed());
                        let user_name = info["user_name"].as_str().unwrap_or("未知");
                        let real_name = info["real_name"].as_str().unwrap_or("");
                        let billing_name = info["billing_name"].as_str().unwrap_or("");
                        if !real_name.is_empty() {
                            println!(
                                "{} {} ({})",
                                "🆔 用户账号:".dimmed(),
                                user_name.cyan().bold(),
                                real_name.green()
                            );
                        } else {
                            println!("{} {}", "🆔 用户账号:".dimmed(), user_name.cyan().bold());
                        }
                        println!("{} {}", "🏢 计费组别:".dimmed(), billing_name.yellow());

                        let balance = info["user_balance"].as_f64().unwrap_or(0.0);
                        let wallet = info["wallet_balance"].as_f64().unwrap_or(0.0);
                        println!(
                            "{} ¥{:.2} (钱包: ¥{:.2})",
                            "💰 账户余额:".dimmed(),
                            balance,
                            wallet
                        );

                        println!("\n{}", "--- 📶 网络信息 ---".dimmed());
                        println!(
                            "{} {}",
                            "🌐 本机 IPv4:".dimmed(),
                            info["online_ip"].as_str().unwrap_or("未知").cyan()
                        );
                        let ip6 = info["online_ip6"].as_str().unwrap_or("::");
                        if ip6 != "::" && !ip6.is_empty() {
                            println!("{} {}", "🌐 本机 IPv6:".dimmed(), ip6.cyan());
                        } else {
                            println!("{} {}", "🌐 本机 IPv6:".dimmed(), "未分配/未开启".dimmed());
                        }
                        println!(
                            "{} {}",
                            "🔗 物理地址:".dimmed(),
                            info["user_mac"].as_str().unwrap_or("未知").dimmed()
                        );

                        // 流量详细换算
                        let total_bytes = info["sum_bytes"]
                            .as_f64()
                            .or_else(|| info["sum_bytes"].as_str().and_then(|s| s.parse().ok()))
                            .unwrap_or(0.0);
                        let session_bytes = info["all_bytes"].as_f64().unwrap_or(0.0);
                        let bytes_in = info["bytes_in"].as_f64().unwrap_or(0.0);
                        let bytes_out = info["bytes_out"].as_f64().unwrap_or(0.0);
                        let remain_bytes = info["remain_bytes"].as_f64().unwrap_or(0.0);

                        let format_flow = |b: f64| {
                            if b >= 1024.0 * 1024.0 * 1024.0 {
                                format!("{:.2} GB", b / 1024.0 / 1024.0 / 1024.0)
                            } else {
                                format!("{:.2} MB", b / 1024.0 / 1024.0)
                            }
                        };

                        println!(
                            "{} {}",
                            "📉 累计流量:".dimmed(),
                            format_flow(total_bytes).cyan().bold()
                        );
                        println!(
                            "{} {} (⬇️ {} / ⬆️ {})",
                            "📊 本次会话:".dimmed(),
                            format_flow(session_bytes).green(),
                            format_flow(bytes_in).dimmed(),
                            format_flow(bytes_out).dimmed()
                        );

                        if remain_bytes > 0.0 {
                            println!(
                                "{} {}",
                                "🎁 剩余流量:".dimmed(),
                                format_flow(remain_bytes).yellow().bold()
                            );
                        }

                        // 时间详细换算
                        let seconds = info["sum_seconds"]
                            .as_u64()
                            .or_else(|| info["sum_seconds"].as_str().and_then(|s| s.parse().ok()))
                            .unwrap_or(0);
                        let remain_seconds = info["remain_seconds"].as_u64().unwrap_or(0);

                        let format_time = |s: u64| {
                            let days = s / 86400;
                            let hours = (s % 86400) / 3600;
                            let minutes = (s % 3600) / 60;
                            let secs = s % 60;
                            if days > 0 {
                                format!("{}天 {}小时 {}分 {}秒", days, hours, minutes, secs)
                            } else {
                                format!("{}小时 {}分 {}秒", hours, minutes, secs)
                            }
                        };

                        println!(
                            "{} {}",
                            "⏱️ 在线时长:".dimmed(),
                            format_time(seconds).cyan().bold()
                        );
                        if remain_seconds > 0 {
                            println!(
                                "{} {}",
                                "⌛ 剩余时长:".dimmed(),
                                format_time(remain_seconds).yellow().bold()
                            );
                        }

                        use chrono::{Local, TimeZone};
                        if let Some(add_time) = info["add_time"].as_u64() {
                            let dt = Local.timestamp_opt(add_time as i64, 0).unwrap();
                            println!(
                                "{} {}",
                                "🕒 登录时间:".dimmed(),
                                dt.format("%Y-%m-%d %H:%M:%S").to_string().dimmed()
                            );
                        }
                        if let Some(keep_time) = info["keepalive_time"].as_u64() {
                            let dt = Local.timestamp_opt(keep_time as i64, 0).unwrap();
                            println!(
                                "{} {}",
                                "💓 最后活跃:".dimmed(),
                                dt.format("%Y-%m-%d %H:%M:%S").to_string().dimmed()
                            );
                        }

                        println!("\n{}", "--- 📋 套餐与设备 ---".dimmed());
                        println!(
                            "{} {} (ID: {})",
                            "💼 订购产品:".dimmed(),
                            info["products_name"].as_str().unwrap_or("未知").yellow(),
                            info["products_id"].as_u64().unwrap_or(0)
                        );

                        // 在线设备详情解析增强
                        let total_dev = info["online_device_total"].as_str().unwrap_or("1");
                        println!(
                            "{} {} 台",
                            "📱 在线设备:".dimmed(),
                            total_dev.magenta().bold()
                        );

                        if let Some(detail_str) = info["online_device_detail"].as_str() {
                            if let Ok(detail) =
                                serde_json::from_str::<serde_json::Value>(detail_str)
                            {
                                if let Some(devices) = detail.as_object() {
                                    for (i, (rad_id, dev)) in devices.iter().enumerate() {
                                        let dev_ip = dev["ip"].as_str().unwrap_or("未知");
                                        let dev_ip6 = dev["ip6"].as_str().unwrap_or("::");
                                        let dev_os = dev["os_name"].as_str().unwrap_or("未知");
                                        let dev_cls = dev["class_name"].as_str().unwrap_or("");
                                        let mark =
                                            if dev_ip == info["online_ip"].as_str().unwrap_or("") {
                                                " (本机)"
                                            } else {
                                                ""
                                            };

                                        println!(
                                            "   {}. {}{} - {} [{}]",
                                            i + 1,
                                            dev_ip.cyan(),
                                            mark.green(),
                                            dev_os.white(),
                                            dev_cls.dimmed()
                                        );
                                        if dev_ip6 != "::" && !dev_ip6.is_empty() {
                                            println!("      └─ IPv6: {}", dev_ip6.dimmed());
                                        }
                                        println!("      └─ 会话ID: {}", rad_id.dimmed());
                                    }
                                }
                            }
                        }

                        println!(
                            "\n{} {}",
                            "🛠️ 系统版本:".dimmed(),
                            info["sysver"].as_str().unwrap_or("未知").dimmed()
                        );
                        if let Some(domain) = info["domain"].as_str() {
                            if !domain.is_empty() {
                                println!("{} {}", "🌐 认证域名:".dimmed(), domain.dimmed());
                            }
                        }
                    }
                }
                Err(e) => eprintln!("❌ 获取状态失败: {}", e),
            }
        }
    }
}
