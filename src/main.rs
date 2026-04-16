mod crypto;
mod api;

use clap::{Parser, Subcommand};
use api::SrunClient;
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
    #[arg(short = 'U', long, env = "SRUN_URL", global = true, default_value = "http://192.168.16.66")]
    url: String,

    /// 校园网账号 (登录时必填)
    #[arg(short = 'u', long, env = "SRUN_USER", global = true)]
    username: Option<String>,

    /// 校园网密码 (登录时必填)
    #[arg(short = 'p', long, env = "SRUN_PASS", global = true)]
    password: Option<String>,

    /// 网关节点 ID (AC ID)，温州大学通常为 2
    #[arg(short = 'a', long, default_value = "2", global = true)]
    ac_id: String,

    /// 强制执行登录 (即使当前已在线)
    #[arg(short, long, global = true)]
    force: bool,

    /// 启用双栈认证 (IPv4/IPv6 Dual Stack)
    #[arg(short = 'd', long, global = true)]
    dual_stack: bool,
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

fn main() {
    let args = Args::parse();

    println!("{}", "\n==========================================".cyan());
    println!("   {} v{}", "🚀 I-WZU-AUTH SRUN CLIENT".bold().bright_yellow(), env!("CARGO_PKG_VERSION").dimmed());
    println!("{}", "==========================================\n".cyan());

    let url = args.url;
    let ac_id = args.ac_id;
    let dual_stack = args.dual_stack;

    match args.command.unwrap_or(Commands::Login) {
        Commands::Login => {
            // 优雅地检查账号密码
            let username = match args.username {
                Some(u) => u,
                None => {
                    eprintln!("{} {} {}\n", "❌".red(), "参数错误:".red().bold(), "登录需要指定账号。".white());
                    eprintln!("{} 请使用 {} 参数或设置 {} 环境变量。\n", "  ".dimmed(), "-u".cyan(), "SRUN_USER".cyan());
                    std::process::exit(1);
                }
            };
            let password = match args.password {
                Some(p) => p,
                None => {
                    eprintln!("{} {} {}\n", "❌".red(), "参数错误:".red().bold(), "登录需要指定密码。".white());
                    eprintln!("{} 请使用 {} 参数或设置 {} 环境变量。\n", "  ".dimmed(), "-p".cyan(), "SRUN_PASS".cyan());
                    std::process::exit(1);
                }
            };
            
            if !args.force {
                print!("{} {} ", "🔍".blue(), "正在检测网络连通性...".white());
                if SrunClient::check_online() {
                    println!("{}", "已联网 (Online)".green().bold());
                    println!("{}\n", "✨ 您已处于在线状态，无需重复登录。".bright_green());
                    return;
                }
                println!("{}", "未联网 (Offline)".yellow());
            }

            let client = SrunClient::new(&url, &username, &password, &ac_id, dual_stack);
            println!("{} {} {}", "🔑".blue(), "正在为用户".white(), username.cyan().bold());
            if dual_stack {
                println!("{} {}", "🌐".blue(), "认证模式: IPv4/IPv6 双栈".white());
            }
            println!("{} {}", "📡".blue(), "发起认证请求...".white());

            match client.login() {
                Ok(resp) => {
                    if resp.res == "ok" {
                        println!("{} {}", "✅".green(), "登录成功！服务器响应: OK".green().bold());
                    } else {
                        let error_msg = resp.error_msg.clone().unwrap_or_else(|| "未知错误".to_string());
                        let error_code = resp.error.clone().unwrap_or_else(|| "N/A".to_string());

                        let is_no_response = resp.res == "no_response_data_error" 
                                          || error_code == "no_response_data_error"
                                          || error_msg.contains("no_response_data_error");

                        eprintln!("\n{} {} {}", "❌".red(), "登录失败:".red().bold(), 
                            if is_no_response { "no_response_data_error".bright_red() } else { error_msg.bright_red() }
                        );

                        if is_no_response {
                            println!("\n{} {} {}", "💡".yellow(), "温馨提示:".yellow().bold(), "检测到 `no_response_data_error`。".white());
                            println!("   {} 您可能正在使用 {} (如 Clash/V2Ray/sing-box TUN 模式)？", "➤".cyan(), "虚拟网卡代理".cyan().bold());
                            println!("   {} 建议 {} 后再重试认证。\n", "➤".cyan(), "先关闭代理".cyan().bold());
                        }
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("\n{} {} {}\n", "🚨".red(), "发生严重错误:".red().bold(), e.bright_red());
                    std::process::exit(1);
                }
            }

            print!("{} {} ", "🔬".blue(), "正在进行二次联网验证".white());
            for i in (1..=3).rev() {
                print!("{} ", i.to_string().cyan());
                io::stdout().flush().unwrap();
                std::thread::sleep(std::time::Duration::from_secs(1));
            }

            if SrunClient::check_online() {
                println!("{}", "验证通过 (Success)".green().bold());
                println!("\n{}\n", "🎉 认证流程全部完成，祝您用网愉快！".bright_green().bold());
            } else {
                println!("{}", "验证失败 (Failed)".red().bold());
                std::process::exit(1);
            }
        }
        Commands::Logout => {
            let client = SrunClient::new(&url, "", "", &ac_id, dual_stack);
            println!("{} {}", "📊".blue(), "正在识别当前在线账号...".white());
            
            match client.check_info("") {
                Ok(info) => {
                    if let Some(online_user) = info["user_name"].as_str() {
                        println!("{} {} {}", "👤".blue(), "检测到在线用户:".white(), online_user.cyan().bold());
                        let logout_client = SrunClient::new(&url, online_user, "", &ac_id, dual_stack);
                        println!("{} {}", "📡".blue(), "正在发起注销请求...".white());
                        match logout_client.logout() {
                            Ok(_) => {
                                println!("{} {}", "✅".green(), "注销成功！计费已停止。".green().bold());
                                // 注销后延迟验证
                                print!("{} {} ", "🔍".blue(), "正在验证断网状态".white());
                                for i in (1..=3).rev() {
                                    print!("{} ", i.to_string().cyan());
                                    io::stdout().flush().unwrap();
                                    std::thread::sleep(std::time::Duration::from_secs(1));
                                }

                                if SrunClient::check_online() {
                                    println!("{}", "仍然在线 (Still Online)".yellow());
                                    println!("{} {}\n", "💡".yellow(), "网关放行可能存在延迟，请等待物理连接自动断开。".dimmed());
                                } else {
                                    println!("{}", "已断开 (Disconnected)".green());
                                }
                            },
                            Err(e) => eprintln!("❌ 注销失败: {}", e),
                        }
                    } else {
                        println!("{} {}", "ℹ️".yellow(), "您当前似乎并不在线，无需注销。".yellow());
                    }
                }
                Err(e) => eprintln!("❌ 无法获取在线信息: {}", e),
            }
        }
        Commands::Status => {
            let client = SrunClient::new(&url, "", "", &ac_id, dual_stack);
            println!("{} {}", "📊".blue(), "正在获取当前在线状态...".white());
            match client.check_info("") {
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
                            println!("{} {} ({})", "🆔 用户账号:".dimmed(), user_name.cyan().bold(), real_name.green());
                        } else {
                            println!("{} {}", "🆔 用户账号:".dimmed(), user_name.cyan().bold());
                        }
                        println!("{} {}", "🏢 计费组别:".dimmed(), billing_name.yellow());
                        
                        let balance = info["user_balance"].as_f64().unwrap_or(0.0);
                        let wallet = info["wallet_balance"].as_f64().unwrap_or(0.0);
                        println!("{} ¥{:.2} (钱包: ¥{:.2})", "💰 账户余额:".dimmed(), balance, wallet);

                        println!("\n{}", "--- 📶 网络信息 ---".dimmed());
                        println!("{} {}", "🌐 本机 IPv4:".dimmed(), info["online_ip"].as_str().unwrap_or("未知").cyan());
                        let ip6 = info["online_ip6"].as_str().unwrap_or("::");
                        if ip6 != "::" && !ip6.is_empty() {
                            println!("{} {}", "🌐 本机 IPv6:".dimmed(), ip6.cyan());
                        } else {
                            println!("{} {}", "🌐 本机 IPv6:".dimmed(), "未分配/未开启".dimmed());
                        }
                        println!("{} {}", "🔗 物理地址:".dimmed(), info["user_mac"].as_str().unwrap_or("未知").dimmed());
                        
                        // 流量详细换算
                        let total_bytes = info["sum_bytes"].as_f64()
                            .or_else(|| info["sum_bytes"].as_str().and_then(|s| s.parse().ok()))
                            .unwrap_or(0.0);
                        let session_bytes = info["all_bytes"].as_f64().unwrap_or(0.0);
                        let bytes_in = info["bytes_in"].as_f64().unwrap_or(0.0);
                        let bytes_out = info["bytes_out"].as_f64().unwrap_or(0.0);
                        let remain_bytes = info["remain_bytes"].as_f64().unwrap_or(0.0);
                        
                        let format_flow = |b: f64| {
                            if b >= 1024.0 * 1024.0 * 1024.0 { format!("{:.2} GB", b / 1024.0 / 1024.0 / 1024.0) }
                            else { format!("{:.2} MB", b / 1024.0 / 1024.0) }
                        };
                        
                        println!("{} {}", "📉 累计流量:".dimmed(), format_flow(total_bytes).cyan().bold());
                        println!("{} {} (⬇️ {} / ⬆️ {})", "📊 本次会话:".dimmed(), format_flow(session_bytes).green(), format_flow(bytes_in).dimmed(), format_flow(bytes_out).dimmed());
                        
                        if remain_bytes > 0.0 {
                            println!("{} {}", "🎁 剩余流量:".dimmed(), format_flow(remain_bytes).yellow().bold());
                        }

                        // 时间详细换算
                        let seconds = info["sum_seconds"].as_u64()
                            .or_else(|| info["sum_seconds"].as_str().and_then(|s| s.parse().ok()))
                            .unwrap_or(0);
                        let remain_seconds = info["remain_seconds"].as_u64().unwrap_or(0);
                        
                        let format_time = |s: u64| {
                            let days = s / 86400;
                            let hours = (s % 86400) / 3600;
                            let minutes = (s % 3600) / 60;
                            let secs = s % 60;
                            if days > 0 { format!("{}天 {}小时 {}分 {}秒", days, hours, minutes, secs) }
                            else { format!("{}小时 {}分 {}秒", hours, minutes, secs) }
                        };
                        
                        println!("{} {}", "⏱️ 在线时长:".dimmed(), format_time(seconds).cyan().bold());
                        if remain_seconds > 0 {
                            println!("{} {}", "⌛ 剩余时长:".dimmed(), format_time(remain_seconds).yellow().bold());
                        }

                        use chrono::{TimeZone, Local};
                        if let Some(add_time) = info["add_time"].as_u64() {
                            let dt = Local.timestamp_opt(add_time as i64, 0).unwrap();
                            println!("{} {}", "🕒 登录时间:".dimmed(), dt.format("%Y-%m-%d %H:%M:%S").to_string().dimmed());
                        }
                        if let Some(keep_time) = info["keepalive_time"].as_u64() {
                            let dt = Local.timestamp_opt(keep_time as i64, 0).unwrap();
                            println!("{} {}", "💓 最后活跃:".dimmed(), dt.format("%Y-%m-%d %H:%M:%S").to_string().dimmed());
                        }

                        println!("\n{}", "--- 📋 套餐与设备 ---".dimmed());
                        println!("{} {} (ID: {})", "💼 订购产品:".dimmed(), info["products_name"].as_str().unwrap_or("未知").yellow(), info["products_id"].as_u64().unwrap_or(0));
                        
                        // 在线设备详情解析增强
                        let total_dev = info["online_device_total"].as_str().unwrap_or("1");
                        println!("{} {} 台", "📱 在线设备:".dimmed(), total_dev.magenta().bold());
                        
                        if let Some(detail_str) = info["online_device_detail"].as_str() {
                            if let Ok(detail) = serde_json::from_str::<serde_json::Value>(detail_str) {
                                if let Some(devices) = detail.as_object() {
                                    for (i, (rad_id, dev)) in devices.iter().enumerate() {
                                        let dev_ip = dev["ip"].as_str().unwrap_or("未知");
                                        let dev_ip6 = dev["ip6"].as_str().unwrap_or("::");
                                        let dev_os = dev["os_name"].as_str().unwrap_or("未知");
                                        let dev_cls = dev["class_name"].as_str().unwrap_or("");
                                        let mark = if dev_ip == info["online_ip"].as_str().unwrap_or("") { " (本机)" } else { "" };
                                        
                                        println!("   {}. {}{} - {} [{}]", i+1, dev_ip.cyan(), mark.green(), dev_os.white(), dev_cls.dimmed());
                                        if dev_ip6 != "::" && !dev_ip6.is_empty() {
                                            println!("      └─ IPv6: {}", dev_ip6.dimmed());
                                        }
                                        println!("      └─ 会话ID: {}", rad_id.dimmed());
                                    }
                                }
                            }
                        }

                        println!("\n{} {}", "🛠️ 系统版本:".dimmed(), info["sysver"].as_str().unwrap_or("未知").dimmed());
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
