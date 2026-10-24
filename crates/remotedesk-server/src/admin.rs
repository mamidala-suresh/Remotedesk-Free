use crate::registry::DeviceRegistry;
use std::sync::Arc;

pub fn render_admin_dashboard(devices_count: usize, sessions_count: usize) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>RemoteDesk Free - Administration Console</title>
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <style>
    :root {{
      --bg: #0d1117;
      --card-bg: #161b22;
      --border: #30363d;
      --accent: #238636;
      --accent-blue: #1f6feb;
      --text: #c9d1d9;
      --text-dim: #8b949e;
      --danger: #da3633;
    }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      background-color: var(--bg);
      color: var(--text);
      margin: 0;
      padding: 24px;
    }}
    .container {{
      max-width: 1100px;
      margin: 0 auto;
    }}
    header {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding-bottom: 20px;
      border-bottom: 1px solid var(--border);
      margin-bottom: 24px;
    }}
    h1 {{
      margin: 0;
      font-size: 24px;
      color: #ffffff;
      display: flex;
      align-items: center;
      gap: 10px;
    }}
    .badge {{
      background: var(--accent);
      color: #fff;
      font-size: 12px;
      padding: 3px 8px;
      border-radius: 12px;
      font-weight: bold;
    }}
    .stats-grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
      gap: 16px;
      margin-bottom: 28px;
    }}
    .card {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 20px;
    }}
    .card-title {{
      font-size: 14px;
      color: var(--text-dim);
      text-transform: uppercase;
      letter-spacing: 0.5px;
      margin-bottom: 8px;
    }}
    .card-value {{
      font-size: 32px;
      font-weight: 700;
      color: #fff;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      margin-top: 12px;
    }}
    th, td {{
      text-align: left;
      padding: 12px 14px;
      border-bottom: 1px solid var(--border);
      font-size: 14px;
    }}
    th {{
      color: var(--text-dim);
      background: #11151c;
      font-weight: 600;
    }}
    tr:hover {{
      background: #1f242c;
    }}
    .status-dot {{
      display: inline-block;
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--accent);
      margin-right: 6px;
    }}
    .refresh-btn {{
      background: var(--accent-blue);
      color: #fff;
      border: none;
      padding: 8px 16px;
      border-radius: 6px;
      cursor: pointer;
      font-weight: 600;
    }}
    .refresh-btn:hover {{
      opacity: 0.9;
    }}
  </style>
</head>
<body>
  <div class="container">
    <header>
      <h1>RemoteDesk Free <span class="badge">Self-Hosted Server</span></h1>
      <button class="refresh-btn" onclick="location.reload()">Refresh Data</button>
    </header>

    <div class="stats-grid">
      <div class="card">
        <div class="card-title">Online Devices</div>
        <div class="card-value">{}</div>
      </div>
      <div class="card">
        <div class="card-title">Active Sessions</div>
        <div class="card-value">{}</div>
      </div>
      <div class="card">
        <div class="card-title">Server Status</div>
        <div class="card-value" style="font-size: 20px; color: var(--accent);"><span class="status-dot"></span>Operational</div>
      </div>
    </div>

    <div class="card" style="margin-bottom: 24px;">
      <h2 style="font-size: 18px; margin-top: 0; color: #fff;">Registered Devices</h2>
      <div id="devices-container">
        <p style="color: var(--text-dim); font-size: 14px;">Loading registered devices...</p>
      </div>
    </div>

    <div class="card">
      <h2 style="font-size: 18px; margin-top: 0; color: #fff;">Active Remote Sessions</h2>
      <div id="sessions-container">
        <p style="color: var(--text-dim); font-size: 14px;">Loading active sessions...</p>
      </div>
    </div>
  </div>

  <script>
    async function fetchServerData() {{
      try {{
        const devRes = await fetch('/api/devices');
        const devices = await devRes.json();
        const devContainer = document.getElementById('devices-container');
        if (devices.length === 0) {{
          devContainer.innerHTML = '<p style="color: var(--text-dim);">No devices currently registered.</p>';
        }} else {{
          let html = '<table><thead><tr><th>Device ID</th><th>Endpoint IP</th><th>OS Info</th><th>Client Version</th><th>Connected At</th></tr></thead><tbody>';
          devices.forEach(d => {{
            html += `<tr><td><strong>${{d.device_id}}</strong></td><td>${{d.peer_addr}}</td><td>${{d.os_info}}</td><td>${{d.app_version}}</td><td>${{new Date(d.connected_at).toLocaleString()}}</td></tr>`;
          }});
          html += '</tbody></table>';
          devContainer.innerHTML = html;
        }}

        const sessRes = await fetch('/api/sessions');
        const sessions = await sessRes.json();
        const sessContainer = document.getElementById('sessions-container');
        if (sessions.length === 0) {{
          sessContainer.innerHTML = '<p style="color: var(--text-dim);">No active sessions currently running.</p>';
        }} else {{
          let html = '<table><thead><tr><th>Session ID</th><th>Initiator</th><th>Host Target</th><th>Started At</th><th>Permissions</th></tr></thead><tbody>';
          sessions.forEach(s => {{
            html += `<tr><td>${{s.session_id}}</td><td>${{s.initiator_id}}</td><td>${{s.target_id}}</td><td>${{new Date(s.started_at).toLocaleString()}}</td><td>Full Access</td></tr>`;
          }});
          html += '</tbody></table>';
          sessContainer.innerHTML = html;
        }}
      }} catch (err) {{
        console.error("Failed to load server data", err);
      }}
    }}
    fetchServerData();
    setInterval(fetchServerData, 5000);
  </script>
</body>
</html>"#,
        devices_count, sessions_count
    )
}

pub async fn handle_admin_api(
    uri_path: &str,
    registry: &Arc<DeviceRegistry>,
) -> Option<(String, String)> {
    match uri_path {
        "/" | "/admin" => {
            let devs = registry.list_devices_summary().await.len();
            let sess = registry.list_sessions().await.len();
            Some(("text/html".to_string(), render_admin_dashboard(devs, sess)))
        }
        "/api/status" => {
            let devs = registry.list_devices_summary().await.len();
            let sess = registry.list_sessions().await.len();
            let json = serde_json::json!({
                "status": "ok",
                "version": "1.0.0",
                "online_devices": devs,
                "active_sessions": sess,
            });
            Some(("application/json".to_string(), json.to_string()))
        }
        "/api/devices" => {
            let devs = registry.list_devices_summary().await;
            Some(("application/json".to_string(), serde_json::to_string(&devs).unwrap()))
        }
        "/api/sessions" => {
            let sessions = registry.list_sessions().await;
            Some(("application/json".to_string(), serde_json::to_string(&sessions).unwrap()))
        }
        _ => None,
    }
}
