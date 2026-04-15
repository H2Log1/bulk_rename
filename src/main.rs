use inquire::{Confirm, Select, Text};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// 历史记录结构体，用于撤销操作
#[derive(Serialize, Deserialize, Debug)]
struct RenameRecord {
    old_path: PathBuf,
    new_path: PathBuf,
}

const HISTORY_FILE: &str = ".rename_history.json";

fn main() -> std::io::Result<()> {
    println!("======================");
    println!("🚀 文件批量重命名 v1.0");
    println!("======================\n");

    loop {
        if let Err(e) = run_once() {
            eprintln!("❌ 操作失败: {}", e);
        }

        let continue_running = Confirm::new("是否继续进行下一次操作？")
            .with_default(true)
            .prompt()
            .unwrap_or(false);

        if !continue_running {
            println!("👋 程序已退出。");
            break;
        }
    }

    Ok(())
}

fn run_once() -> std::io::Result<()> {
    let has_history = Path::new(HISTORY_FILE).exists();

    // 1. 功能选择
    let mut options = vec![
        "添加前缀",
        "搜索并替换",
        "正则表达式替换",
        "序列自动编号",
    ];
    if has_history {
        options.push("🔙 撤销上次重命名操作");
    }

    let strategy = Select::new("请选择操作模式:", options)
        .prompt()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // 处理撤销逻辑
    if strategy == "🔙 撤销上次重命名操作" {
        perform_undo()?;
        return Ok(());
    }

    // 2. 获取目标路径
    let default_target_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|dir| dir.to_path_buf()))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    let target_dir = Text::new("请输入目标文件夹路径:")
        .with_default(default_target_dir.to_string_lossy().as_ref())
        .prompt()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let path = Path::new(&target_dir);
    if !path.exists() || !path.is_dir() {
        println!("❌ 错误：路径无效！");
        return Ok(());
    }

    // 3. 配置重命名参数
    let mut search_pattern = String::new();
    let mut replace_template = String::new();

    match strategy {
        "添加前缀" => {
            replace_template = Text::new("请输入前缀:").prompt().unwrap();
        }
        "搜索并替换" => {
            search_pattern = Text::new("搜索内容:").prompt().unwrap();
            replace_template = Text::new("替换内容:").prompt().unwrap();
        }
        "正则表达式替换" => {
            search_pattern = Text::new("正则模式 (如 (\\d+)-img):").prompt().unwrap();
            replace_template = Text::new("替换模板 (如 $1_new):").prompt().unwrap();
        }
        _ => {}
    }

    // 4. 扫描文件并生成预览
    let entries: Vec<PathBuf> = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();

    if entries.is_empty() {
        println!("⚠️ 目录下无文件。");
        return Ok(());
    }

    let re = if strategy == "正则表达式替换" {
        Some(Regex::new(&search_pattern).expect("正则语法错误"))
    } else {
        None
    };

    println!("\n--- 重命名预览 (前5个样例) ---");
    let mut preview_list = Vec::new();
    for (i, old_path) in entries.iter().enumerate() {
        let file_name = old_path.file_name().and_then(|s| s.to_str()).unwrap();
        let new_name = match strategy {
            "添加前缀" => format!("{}{}", replace_template, file_name),
            "搜索并替换" => file_name.replace(&search_pattern, &replace_template),
            "正则表达式替换" => re.as_ref().unwrap().replace_all(file_name, &replace_template).to_string(),
            "序列自动编号" => {
                let ext = old_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if ext.is_empty() { format!("{}", i + 1) } else { format!("{}.{}", i + 1, ext) }
            }
            _ => file_name.to_string(),
        };
        if i < 5 { println!("  {}  ->  {}", file_name, new_name); }
        preview_list.push(new_name);
    }

    // 5. 确认并执行
    if !Confirm::new("确认执行上述重命名？").with_default(false).prompt().unwrap() {
        println!("🚫 已取消。");
        return Ok(());
    }

    let mut history = Vec::new();
    for (i, old_path) in entries.iter().enumerate() {
        let new_name = &preview_list[i];
        if new_name != old_path.file_name().and_then(|s| s.to_str()).unwrap() {
            let mut new_path = old_path.clone();
            new_path.set_file_name(new_name);

            if let Ok(_) = fs::rename(old_path, &new_path) {
                history.push(RenameRecord {
                    old_path: old_path.clone(),
                    new_path: new_path,
                });
            }
        }
    }

    // 6. 保存历史记录
    if !history.is_empty() {
        let json = serde_json::to_string(&history).unwrap();
        fs::write(HISTORY_FILE, json)?;
        println!("\n✅ 成功重命名 {} 个文件。", history.len());
        println!("💡 提示：如需撤销，请再次运行本程序并选择“撤销”选项。");
    } else {
        println!("\nℹ️ 没有需要重命名的文件。");
    }

    Ok(())
}

fn perform_undo() -> std::io::Result<()> {
    let data = fs::read_to_string(HISTORY_FILE)?;
    let history: Vec<RenameRecord> = serde_json::from_str(&data).unwrap();

    println!("正在撤销上次操作...");
    let mut count = 0;
    for record in history {
        if record.new_path.exists() {
            fs::rename(&record.new_path, &record.old_path)?;
            count += 1;
        }
    }
    fs::remove_file(HISTORY_FILE)?;
    println!("✨ 成功还原 {} 个文件！", count);
    Ok(())
}