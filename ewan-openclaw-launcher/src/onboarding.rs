/// onboarding.rs
/// 首次引导模块
///
/// 不依赖 WebView2，改用本地 HTTP 服务器 + 系统默认浏览器：
/// 1. 绑定本地随机端口，启动临时 HTTP 服务
/// 2. 用系统浏览器打开引导页（localhost:PORT）
/// 3. 用户填写 API 配置后提交，本地服务接收 POST 数据
/// 4. 写入配置后关闭临时服务

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

use tracing::{info, warn};

/// 用户填写的引导配置
#[derive(Debug, Clone)]
pub struct OnboardingConfig {
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
    /// API 协议：openai 或 anthropic-messages
    pub api_protocol: String,
    /// WebChat 访问端口，默认 17789
    pub webchat_port: u16,
    /// Windows 侧 workspace 路径，例如 D:\MyWorkspace
    /// 留空则使用默认路径（WSL 内部）
    pub workspace_path: String,
}

/// 引导页 HTML（内嵌在二进制里，纯静态，无需任何外部依赖）
const ONBOARDING_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>AI 助手 - 初始配置</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: -apple-system, "Microsoft YaHei", sans-serif;
    background: #f5f7fa;
    display: flex; align-items: center; justify-content: center;
    min-height: 100vh; padding: 24px;
  }
  .card {
    background: white; border-radius: 12px; padding: 36px;
    width: 100%; max-width: 480px;
    box-shadow: 0 4px 24px rgba(0,0,0,0.08);
  }
  .logo { font-size: 32px; margin-bottom: 8px; }
  h1 { font-size: 20px; font-weight: 600; color: #1a1a1a; margin-bottom: 6px; }
  .subtitle { font-size: 14px; color: #666; margin-bottom: 28px; line-height: 1.5; }
  .form-group { margin-bottom: 18px; }
  label { display: block; font-size: 13px; font-weight: 500; color: #333; margin-bottom: 6px; }
  .hint { font-size: 12px; color: #999; margin-top: 4px; }
  input {
    width: 100%; padding: 10px 12px; border: 1px solid #ddd;
    border-radius: 8px; font-size: 14px; color: #333;
    transition: border-color 0.2s; outline: none;
  }
  input:focus { border-color: #4f7cff; box-shadow: 0 0 0 3px rgba(79,124,255,0.1); }
  .btn-submit {
    width: 100%; padding: 12px; background: #4f7cff; color: white;
    border: none; border-radius: 8px; font-size: 15px; font-weight: 500;
    cursor: pointer; margin-top: 8px; transition: background 0.2s;
  }
  .btn-submit:hover { background: #3d6ae8; }
  .btn-submit:disabled { background: #b0c0f0; cursor: not-allowed; }
  .error { color: #e53e3e; font-size: 13px; margin-top: 8px; display: none; }
  .success {
    text-align: center; padding: 32px 0; display: none;
  }
  .success .icon { font-size: 48px; margin-bottom: 12px; }
  .success h2 { font-size: 18px; color: #1a1a1a; margin-bottom: 8px; }
  .success p { font-size: 14px; color: #666; }
  .restart-hint {
    margin-top: 16px; padding: 12px; background: #fff7e6;
    border: 1px solid #ffd591; border-radius: 8px; font-size: 13px; color: #663c00;
  }
  .restart-hint code {
    display: block; margin-top: 6px; padding: 6px 8px; background: #fff;
    border-radius: 4px; font-family: monospace; font-size: 12px;
    word-break: break-all;
  }
</style>
</head>
<body>
<div class="card">
  <div id="formView">
    <div class="logo">🤖</div>
    <h1>AI 助手配置</h1>
    <p class="subtitle">配置 AI 模型信息，修改配置后需手动执行脚本使配置生效。</p>
    <div class="form-group">
      <label for="apiProtocol">API 协议 <span style="color:#e53e3e">*</span></label>
      <select id="apiProtocol">
        <option value="openai">OpenAI 兼容协议（OpenAI / DeepSeek / Moonshot 等）</option>
        <option value="anthropic-messages">Anthropic 协议（Claude 原生 / 兼容中转）</option>
      </select>
      <div class="hint">不确定选哪个？大多数国内模型选 OpenAI 兼容协议</div>
    </div>
    <div class="form-group">
      <label for="baseUrl">API Base URL</label>
      <input type="text" id="baseUrl" placeholder="https://api.openai.com/v1" />
      <div class="hint">API 服务地址，末尾不需要加斜杠</div>
    </div>
    <div class="form-group">
      <label for="apiKey">API Key <span style="color:#e53e3e">*</span></label>
      <input type="password" id="apiKey" placeholder="sk-..." />
      <div class="hint">您的 API 密钥，仅保存在本机</div>
    </div>
    <div class="form-group">
      <label for="modelName">模型名称</label>
      <input type="text" id="modelName" placeholder="gpt-4o" />
      <div class="hint">留空则使用提供商默认模型</div>
    </div>
    <hr style="border:none;border-top:1px solid #eee;margin:20px 0">
    <div class="form-group">
      <label for="webchatPort">WebChat 端口</label>
      <input type="number" id="webchatPort" value="17789" min="1024" max="65535" />
      <div class="hint">AI 助手网页界面的访问端口，默认 17789。如与其他程序冲突可修改。</div>
    </div>
    <div class="form-group">
      <label for="workspacePath">Workspace 路径（可选）</label>
      <input type="text" id="workspacePath" placeholder="D:\MyWorkspace" />
      <div class="hint">AI 助手的工作目录，留空则使用默认路径。填写后将自动挂载到 WSL 内部。</div>
    </div>
    <div class="error" id="errorMsg"></div>
    <button class="btn-submit" id="submitBtn" onclick="submit()">确认并开始使用</button>
  </div>
  <div class="success" id="successView">
    <div class="icon">✅</div>
    <h2>配置已保存</h2>
    <p style="margin-top:12px;font-size:14px;color:#666">配置已保存到本地。</p>
    <div class="restart-hint">
      ⚠️ 配置已更新，需要手动执行脚本使配置生效：<br>
      <code id="restartPath"></code>
    </div>
  </div>
</div>
<script>
// 页面加载时读取已有配置回显
(async function() {
  try {
    const resp = await fetch('/config');
    if (resp.ok) {
      const cfg = await resp.json();
      if (cfg.baseUrl) document.getElementById('baseUrl').value = cfg.baseUrl;
      if (cfg.apiKey) document.getElementById('apiKey').value = cfg.apiKey;
      if (cfg.modelName) document.getElementById('modelName').value = cfg.modelName;
      if (cfg.apiProtocol) document.getElementById('apiProtocol').value = cfg.apiProtocol;
      if (cfg.webchatPort) document.getElementById('webchatPort').value = cfg.webchatPort;
      if (cfg.workspacePath) document.getElementById('workspacePath').value = cfg.workspacePath;
    }
  } catch(e) { /* 忽略错误，可能是首次使用 */ }
})();

function showError(msg) {
  const el = document.getElementById('errorMsg');
  el.textContent = msg;
  el.style.display = msg ? 'block' : 'none';
}

async function submit() {
  const baseUrl       = document.getElementById('baseUrl').value.trim();
  const apiKey        = document.getElementById('apiKey').value.trim();
  const modelName     = document.getElementById('modelName').value.trim();
  const apiProtocol   = document.getElementById('apiProtocol').value;
  const workspacePath = document.getElementById('workspacePath').value.trim();
  const webchatPort   = parseInt(document.getElementById('webchatPort').value) || 17789;
  if (!apiKey) { showError('请填写 API Key'); return; }
  if (webchatPort < 1024 || webchatPort > 65535) { showError('端口号需在 1024 - 65535 之间'); return; }
  showError('');
  document.getElementById('submitBtn').disabled = true;
  document.getElementById('submitBtn').textContent = '保存中...';
  try {
    const resp = await fetch('/submit', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ baseUrl, apiKey, modelName, apiProtocol, workspacePath, webchatPort })
    });
    if (resp.ok) {
      document.getElementById('formView').style.display = 'none';
      document.getElementById('successView').style.display = 'block';
      // 显示 restart.bat 路径
      const exeDir = location.pathname.substring(0, location.pathname.lastIndexOf('/'));
      // 从当前 URL 推断 exe 所在目录（去掉 / 或者 /config）
      // 由于是本地服务，尝试从 referer 获取或使用相对路径
      const restartPath = 'D:\\EwanOpenClaw\\scripts\\restart.bat';
      document.getElementById('restartPath').textContent = restartPath;
    } else {
      showError('提交失败，请重试');
      document.getElementById('submitBtn').disabled = false;
      document.getElementById('submitBtn').textContent = '确认并开始使用';
    }
  } catch(e) {
    showError('网络错误，请重试');
    document.getElementById('submitBtn').disabled = false;
    document.getElementById('submitBtn').textContent = '确认并开始使用';
  }
}
</script>
</body>
</html>"#;

/// 配置页服务句柄，持有端口号和回调通道
pub struct OnboardingServer {
    pub port: u16,
    pub config_rx: std::sync::mpsc::Receiver<OnboardingConfig>,
}

impl OnboardingServer {
    /// 启动常驻配置页 HTTP 服务（后台线程），返回服务句柄
    ///
    /// 服务会一直运行，每次收到 POST /submit 就通过 channel 推送配置，
    /// 调用方可以随时 try_recv() 检查是否有新配置提交。
    pub fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .context("无法绑定本地端口")?;
        let port = listener.local_addr()?.port();

        let (tx, rx) = std::sync::mpsc::channel::<OnboardingConfig>();

        info!("Onboarding server listening at http://127.0.0.1:{}", port);

        std::thread::spawn(move || {
            listener.set_nonblocking(false).ok();
            loop {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(false).ok();
                        if let Some(config) = handle_request_inner(stream) {
                            info!("New config submitted: base_url={}, model={}", config.base_url, config.model_name);
                            let _ = tx.send(config);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to accept connection: {}", e);
                    }
                }
            }
        });

        Ok(Self { port, config_rx: rx })
    }

    /// 打开系统浏览器访问配置页
    pub fn open(&self) {
        let url = format!("http://127.0.0.1:{}", self.port);
        open_browser(&url);
    }

    /// 尝试接收新提交的配置（非阻塞）
    pub fn try_recv(&self) -> Option<OnboardingConfig> {
        self.config_rx.try_recv().ok()
    }
}


/// 处理一个 HTTP 请求，收到有效 POST /submit 时返回 Some(config)
fn handle_request_inner(mut stream: TcpStream) -> Option<OnboardingConfig> {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();

    if reader.read_line(&mut request_line).is_err() {
        return None;
    }

    // 读取所有请求头
    let mut headers = Vec::new();
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() { break; }
        let line = line.trim().to_string();
        if line.is_empty() { break; }
        if line.to_lowercase().starts_with("content-length:") {
            if let Ok(n) = line[15..].trim().parse::<usize>() {
                content_length = n;
            }
        }
        headers.push(line);
    }

    let request_line = request_line.trim();

    if request_line.starts_with("GET / ") || request_line.starts_with("GET /\r") {
        // 返回引导页 HTML
        let body = ONBOARDING_HTML;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(), body
        );
        let _ = stream.write_all(response.as_bytes());
        return None;
    }

    // 返回当前配置（供页面回显）
    if request_line.starts_with("GET /config ") || request_line.starts_with("GET /config\r") {
        let config = load_temp_config();
        let resp_body = serde_json::to_string(&config).unwrap_or_else(|_| "{}".to_string());
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
            resp_body.len(), resp_body
        );
        let _ = stream.write_all(response.as_bytes());
        return None;
    }

    if request_line.starts_with("POST /submit ") {
        // 读取请求体
        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            use std::io::Read;
            let _ = reader.read_exact(&mut body);
        }

        // 先解析 JSON，再构建响应（这样可以把真实端口回传给前端）
        if let Ok(body_str) = std::str::from_utf8(&body) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body_str) {
                let port = json["webchatPort"].as_u64().unwrap_or(17789) as u16;
                // 读取 gateway token（用于 webchat URL 免 token 弹窗）
                // auth.mode=none，无需 token
                let gateway_token = String::new();
                // 把实际端口和 token 回传给前端
                let resp_body = format!(r#"{{"ok":true,"gatewayPort":{},"gatewayToken":"{}"}}"#, port, gateway_token);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
                    resp_body.len(), resp_body
                );
                let _ = stream.write_all(response.as_bytes());
                return Some(OnboardingConfig {
                    base_url:       json["baseUrl"].as_str().unwrap_or("").to_string(),
                    api_key:        json["apiKey"].as_str().unwrap_or("").to_string(),
                    model_name:     json["modelName"].as_str().unwrap_or("").to_string(),
                    api_protocol:   json["apiProtocol"].as_str().unwrap_or("openai").to_string(),
                    webchat_port:   port,
                    workspace_path: json["workspacePath"].as_str().unwrap_or("").to_string(),
                });
            }
        }
        // JSON 解析失败
        let err = r#"{"ok":false}"#;
        let _ = stream.write_all(format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            err.len(), err
        ).as_bytes());
        return None;
    }

    // 其他请求返回 404
    let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    None
}

/// 用系统默认浏览器打开指定 URL
pub fn open_browser(url: &str) {
    info!("Opening browser: {}", url);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .creation_flags(0x0800_0000)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

/// 显示错误弹窗
pub fn show_error(title: &str, message: &str) {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let title_wide = to_wide(title);
        let msg_wide = to_wide(message);
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(msg_wide.as_ptr()),
                PCWSTR(title_wide.as_ptr()),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("[错误] {}: {}", title, message);
    }
}

/// 从 temp_config.bat 读取当前配置
fn load_temp_config() -> serde_json::Value {
    // 查找 temp_config.bat 路径：{exe_dir}\config\temp_config.bat
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let config_file = exe_dir.join("config").join("temp_config.bat");

    if !config_file.exists() {
        return serde_json::json!({
            "baseUrl": "",
            "apiKey": "",
            "modelName": "",
            "apiProtocol": "openai",
            "webchatPort": 17789,
            "workspacePath": ""
        });
    }

    // 解析 batch 文件中的 set 语句
    let content = std::fs::read_to_string(&config_file).unwrap_or_default();
    let mut base_url = String::new();
    let mut api_key = String::new();
    let mut model_name = String::new();
    let mut api_protocol = String::from("openai");
    let mut webchat_port = 17789u16;
    let mut workspace_path = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("set BASE_URL=") {
            base_url = line[13..].trim_matches('"').to_string();
        } else if line.starts_with("set API_KEY=") {
            api_key = line[12..].trim_matches('"').to_string();
        } else if line.starts_with("set MODEL_NAME=") {
            model_name = line[15..].trim_matches('"').to_string();
        } else if line.starts_with("set API_PROTOCOL=") {
            api_protocol = line[17..].trim_matches('"').to_string();
        } else if line.starts_with("set WEBCHAT_PORT=") {
            if let Ok(p) = line[17..].trim_matches('"').parse::<u16>() {
                webchat_port = p;
            }
        } else if line.starts_with("set WORKSPACE_PATH=") {
            workspace_path = line[19..].trim_matches('"').to_string();
        }
    }

    serde_json::json!({
        "baseUrl": base_url,
        "apiKey": api_key,
        "modelName": model_name,
        "apiProtocol": api_protocol,
        "webchatPort": webchat_port,
        "workspacePath": workspace_path
    })
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}
