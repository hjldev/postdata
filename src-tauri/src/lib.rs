mod engine;
mod model;
mod storage;
#[cfg(test)]
mod tests;
use model::*;
use reqwest_cookie_store::CookieStoreMutex;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
use tokio_util::sync::CancellationToken;

struct AppState {
    client: reqwest::Client,
    jar: Arc<CookieStoreMutex>,
    dir: PathBuf,
    data: Mutex<()>,
    active: Mutex<HashMap<String, CancellationToken>>,
    startup_error: Option<String>,
}
impl AppState {
    fn new(dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut jar = cookie_store::CookieStore::default();
        let mut startup_error = None;
        let cookie_path = dir.join("cookies.json");
        if cookie_path.exists() {
            let parsed = (|| -> Result<cookie_store::CookieStore, String> {
                let value: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(&cookie_path).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                if value["version"] != 1 {
                    return Err("不支持的 Cookie 数据版本".into());
                }
                serde_json::from_value(value["cookies"].clone()).map_err(|e| e.to_string())
            })();
            match parsed {
                Ok(saved) => jar = saved,
                Err(e) => {
                    startup_error =
                        Some(format!("Cookie 加载失败：{e}；原文件保留，暂不保存 Cookie"))
                }
            }
        }
        let jar = Arc::new(CookieStoreMutex::new(jar));
        let client = reqwest::Client::builder()
            .no_proxy()
            .cookie_provider(jar.clone())
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            client,
            jar,
            dir,
            data: Mutex::new(()),
            active: Mutex::new(HashMap::new()),
            startup_error,
        })
    }
    fn save_cookies(&self) -> Result<(), String> {
        if let Some(error) = &self.startup_error {
            return Err(error.clone());
        }
        let jar = self.jar.lock().map_err(|e| e.to_string())?;
        let value = serde_json::json!({"version":1,"cookies": &*jar});
        storage::atomic_write(
            &self.dir.join("cookies.json"),
            &serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
        )
    }
}
#[tauri::command]
async fn send_request(
    state: tauri::State<'_, AppState>,
    request: RequestSpec,
) -> Result<SendOutcome, String> {
    let token = CancellationToken::new();
    {
        let mut active = state.active.lock().map_err(|e| e.to_string())?;
        if !active.is_empty() {
            return Err("已有请求正在发送".into());
        }
        active.insert(request.id.clone(), token.clone());
    }
    let started = Instant::now();
    let result = engine::execute(&state.client, &request, token).await;
    state
        .active
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&request.id);
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let history = History {
        id: request.id.clone(),
        at,
        request,
        status: result.as_ref().ok().map(|r| r.status),
        elapsed_ms: started.elapsed().as_millis() as u64,
        error: result.as_ref().err().cloned(),
    };
    let saved = (|| -> Result<(), String> {
        let _guard = state.data.lock().map_err(|e| e.to_string())?;
        let path = state.dir.join("workspace.json");
        storage::record_history(&path, history)?;
        state.save_cookies()
    })();
    Ok(SendOutcome {
        response: result.as_ref().ok().cloned(),
        error: result.err(),
        storage_error: saved.err(),
    })
}
#[tauri::command]
fn cancel_request(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    if let Some(token) = state.active.lock().map_err(|e| e.to_string())?.get(&id) {
        token.cancel();
    }
    Ok(())
}
#[tauri::command]
fn load_workspace(state: tauri::State<AppState>) -> Result<SavedData, String> {
    let _guard = state.data.lock().map_err(|e| e.to_string())?;
    storage::load(&state.dir.join("workspace.json"))
}
#[tauri::command]
fn save_favorite(state: tauri::State<AppState>, request: RequestSpec) -> Result<(), String> {
    let _guard = state.data.lock().map_err(|e| e.to_string())?;
    let path = state.dir.join("workspace.json");
    let mut data = storage::load(&path)?;
    if let Some(item) = data.favorites.iter_mut().find(|x| x.id == request.id) {
        *item = request;
    } else {
        data.favorites.insert(0, request);
    }
    storage::save(&path, &data)
}
#[tauri::command]
fn delete_favorite(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    let _guard = state.data.lock().map_err(|e| e.to_string())?;
    let path = state.dir.join("workspace.json");
    let mut data = storage::load(&path)?;
    data.favorites.retain(|r| r.id != id);
    storage::save(&path, &data)
}
#[tauri::command]
fn clear_history(state: tauri::State<AppState>) -> Result<(), String> {
    let _guard = state.data.lock().map_err(|e| e.to_string())?;
    let path = state.dir.join("workspace.json");
    let mut data = storage::load(&path)?;
    data.history.clear();
    storage::save(&path, &data)
}
#[tauri::command]
fn list_cookies(state: tauri::State<AppState>) -> Result<CookieList, String> {
    let jar = state.jar.lock().map_err(|e| e.to_string())?;
    let mut cookies = jar
        .iter_unexpired()
        .map(|c| CookieInfo {
            name: c.name().to_owned(),
            value: c.value().to_owned(),
            domain: c.domain.as_cow().unwrap_or_default().into_owned(),
            path: c.path.to_string(),
            secure: c.secure().unwrap_or(false),
            http_only: c.http_only().unwrap_or(false),
            expires_at: match c.expires {
                cookie_store::CookieExpiration::AtUtc(at) => Some(at.unix_timestamp()),
                cookie_store::CookieExpiration::SessionEnd => None,
            },
        })
        .collect::<Vec<_>>();
    cookies.sort_by(|a, b| (&a.domain, &a.path, &a.name).cmp(&(&b.domain, &b.path, &b.name)));
    Ok(CookieList {
        cookies,
        warning: state.startup_error.clone(),
    })
}
#[tauri::command]
fn request_cookie_header(state: tauri::State<AppState>, url: String) -> Result<String, String> {
    let url = reqwest::Url::parse(url.trim()).map_err(|e| format!("无效 URL：{e}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("仅支持 HTTP 和 HTTPS URL".into());
    }
    let jar = state.jar.lock().map_err(|e| e.to_string())?;
    Ok(jar
        .get_request_values(&url)
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; "))
}
#[tauri::command]
fn delete_cookie(
    state: tauri::State<AppState>,
    domain: String,
    path: String,
    name: String,
) -> Result<(), String> {
    state
        .jar
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&domain, &path, &name);
    state.save_cookies()
}
#[tauri::command]
fn clear_cookies(state: tauri::State<AppState>) -> Result<(), String> {
    state.jar.lock().map_err(|e| e.to_string())?.clear();
    state.save_cookies()
}
#[tauri::command]
async fn check_files(paths: Vec<String>) -> Vec<String> {
    let mut missing = vec![];
    for path in paths {
        if !tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_file())
            || tokio::fs::File::open(&path).await.is_err()
        {
            missing.push(path);
        }
    }
    missing
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let state = AppState::new(app.path().app_data_dir()?).map_err(std::io::Error::other)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            send_request,
            cancel_request,
            load_workspace,
            save_favorite,
            delete_favorite,
            clear_history,
            list_cookies,
            request_cookie_header,
            delete_cookie,
            clear_cookies,
            check_files
        ])
        .run(tauri::generate_context!())
        .expect("Postdata 启动失败");
}
