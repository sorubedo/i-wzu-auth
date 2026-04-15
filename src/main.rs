mod crypto;
mod api;

use clap::{Parser, Subcommand};
use api::SrunClient;
use colored::*;

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

            let client = SrunClient::new(&url, &username, &password, &ac_id);
            println!("{} {} {}", "🔑".blue(), "正在为用户".white(), username.cyan().bold());
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

            print!("{} {} ", "🔬".blue(), "正在进行二次联网验证...".white());
            if SrunClient::check_online() {
                println!("{}", "验证通过 (Success)".green().bold());
                println!("\n{}\n", "🎉 认证流程全部完成，祝您用网愉快！".bright_green().bold());
            } else {
                println!("{}", "验证失败 (Failed)".red().bold());
                std::process::exit(1);
            }
        }
        Commands::Logout => {
            let client = SrunClient::new(&url, "", "", &ac_id);
            println!("{} {}", "📊".blue(), "正在识别当前在线账号...".white());
            
            match client.check_info("0.0.0.0") {
                Ok(info) => {
                    if let Some(online_user) = info["user_name"].as_str() {
                        println!("{} {} {}", "👤".blue(), "检测到在线用户:".white(), online_user.cyan().bold());
                        let logout_client = SrunClient::new(&url, online_user, "", &ac_id);
                        println!("{} {}", "📡".blue(), "正在发起注销请求...".white());
                        match logout_client.logout() {
                            Ok(_) => {
                                println!("{} {}", "✅".green(), "注销成功！计费已停止。".green().bold());
                                // 注销后延迟验证
                                print!("{} {} ", "🔍".blue(), "正在验证断网状态...".white());
                                std::thread::sleep(std::time::Duration::from_secs(1));
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
            let client = SrunClient::new(&url, "", "", &ac_id);
            println!("{} {}", "📊".blue(), "正在获取当前在线状态...".white());
            match client.check_info("0.0.0.0") {
                Ok(info) => {
                    if info["error"].as_str() == Some("not_online_error") {
                        println!("{} {}", "ℹ️".yellow(), "状态: 当前未在线".yellow().bold());
                    } else {
                        println!("{} {}", "🟢".green(), "状态: 已在线".green().bold());
                        println!("\n{}", "--- 详细信息 ---".dimmed());
                        println!("{} {}", "👤 用户名:".dimmed(), info["user_name"].as_str().unwrap_or("未知").cyan());
                        println!("{} {}", "🌐 本机 IP:".dimmed(), info["online_ip"].as_str().unwrap_or("未知").cyan());
                        let used_mb = info["sum_bytes"].as_f64().unwrap_or(0.0) / 1024.0 / 1024.0;
                        println!("{} {:.2} MB", "📉 已用流量:".dimmed(), used_mb.to_string().cyan());
                        println!("{} {} 秒", "⏱️ 在线时长:".dimmed(), info["sum_seconds"].to_string().cyan());
                        println!("{} {}", "🏢 网关 IP:".dimmed(), info["nas_ip"].as_str().unwrap_or("不知道喵~").cyan());
                    }
                }
                Err(e) => eprintln!("❌ 获取状态失败: {}", e),
            }
        }
    }
}
