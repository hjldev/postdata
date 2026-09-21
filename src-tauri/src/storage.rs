use crate::model::{History, SavedData};
use std::{fs, path::Path};
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| format!("保存失败：{e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    }
    fs::rename(tmp, path).map_err(|e| format!("保存失败：{e}"))
}
pub fn load(path: &Path) -> Result<SavedData, String> {
    if !path.exists() {
        return Ok(SavedData::default());
    }
    let data: SavedData = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("本地数据损坏，未覆盖原文件：{e}"))?;
    if data.version != 1 {
        return Err("不支持的本地数据版本".into());
    }
    Ok(data)
}
pub fn save(path: &Path, data: &SavedData) -> Result<(), String> {
    atomic_write(
        path,
        &serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?,
    )
}

pub fn record_history(path: &Path, history: History) -> Result<(), String> {
    let mut data = load(path)?;
    data.history.insert(0, history);
    data.history.truncate(200);
    save(path, &data)
}
