//! AutoCAD (ObjectARX プラグイン) への薄いブリッジ。
//!
//! 実体は unix domain socket 経由（プラグインが `/tmp/gfp-arx-bridge.sock` を listen）。
//! unix 固有の実装は `#[cfg(unix)]` の [`send`] に閉じ込め、その他 OS では明示スタブを
//! 返す（ビルドは全 OS で通す — Windows でも cad-mcp 等をローカルビルドできるように）。

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Response {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub success: bool,
    #[allow(dead_code)]
    pub data: serde_json::Value,
}

/// AutoCAD ブリッジに 1 リクエスト送って 1 レスポンスを受ける（unix domain socket）。
#[cfg(unix)]
pub fn send(method: &str, params: serde_json::Value) -> Result<Response> {
    use anyhow::Context;
    use serde::Serialize;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    const SOCKET_PATH: &str = "/tmp/gfp-arx-bridge.sock";
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    #[derive(Serialize)]
    struct Request {
        id: String,
        method: String,
        params: serde_json::Value,
    }

    let mut stream = UnixStream::connect(SOCKET_PATH).context("AutoCAD に接続できません")?;
    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let req = Request {
        id: format!("r_{}", COUNTER.fetch_add(1, Ordering::Relaxed)),
        method: method.to_string(),
        params,
    };

    let mut msg = serde_json::to_string(&req)?;
    msg.push('\n');
    stream.write_all(msg.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    serde_json::from_str(line.trim()).context("レスポンスパース失敗")
}

/// 非 unix では unix domain socket が使えないため未対応（ビルドは通す）。
#[cfg(not(unix))]
pub fn send(_method: &str, _params: serde_json::Value) -> Result<Response> {
    anyhow::bail!("AutoCAD ブリッジは unix domain socket を使うため、この OS では未対応です")
}

pub fn is_connected() -> bool {
    send("ping", serde_json::Value::Null).is_ok()
}
