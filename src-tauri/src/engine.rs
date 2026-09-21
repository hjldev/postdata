use crate::model::*;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Client, Method, Url,
};
use std::{
    str::FromStr,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
pub const MAX_BODY: usize = 10 * 1024 * 1024;

pub async fn execute(
    client: &Client,
    spec: &RequestSpec,
    cancel: CancellationToken,
) -> Result<ResponseData, String> {
    if spec.timeout_ms == 0 || spec.timeout_ms > 600_000 {
        return Err("超时必须在 1–600000 毫秒之间".into());
    }
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err("请求已取消".into()),
        result = tokio::time::timeout(Duration::from_millis(spec.timeout_ms), execute_inner(client, spec)) => {
            result.map_err(|_| "请求超时".to_string())?
        }
    }
}
async fn execute_inner(client: &Client, spec: &RequestSpec) -> Result<ResponseData, String> {
    let started = Instant::now();
    let mut url = Url::parse(&spec.url).map_err(|e| format!("无效 URL：{e}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("仅支持 HTTP 和 HTTPS URL".into());
    }
    // The URL is canonical in the UI; params is its editable representation, not appended twice.
    url.set_fragment(None);
    if !["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"].contains(&spec.method.as_str())
    {
        return Err("不支持的请求方法".into());
    }
    let method = Method::from_str(&spec.method).map_err(|e| e.to_string())?;
    let mut headers = HeaderMap::new();
    for pair in spec
        .headers
        .iter()
        .filter(|p| p.enabled && !p.key.is_empty())
    {
        let name = HeaderName::from_str(&pair.key).map_err(|e| format!("无效请求头名称：{e}"))?;
        let value = HeaderValue::from_str(&pair.value).map_err(|e| format!("无效请求头值：{e}"))?;
        headers.append(name, value);
    }
    if spec.auth.kind != "none" {
        headers.remove(AUTHORIZATION);
    }
    if spec.body.kind == "multipart" {
        headers.remove(CONTENT_TYPE);
    }
    if !headers.contains_key(CONTENT_TYPE) {
        let content = match spec.body.kind.as_str() {
            "json" => Some("application/json"),
            "text" => Some("text/plain; charset=utf-8"),
            "form" => Some("application/x-www-form-urlencoded"),
            _ => None,
        };
        if let Some(content) = content {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static(content));
        }
    }
    let mut request = client.request(method, url).headers(headers);
    request = match spec.auth.kind.as_str() {
        "bearer" => request.bearer_auth(&spec.auth.token),
        "basic" => request.basic_auth(&spec.auth.username, Some(&spec.auth.password)),
        "none" => request,
        _ => return Err("未知鉴权类型".into()),
    };
    request = match spec.body.kind.as_str() {
        "none" => request,
        "json" => {
            serde_json::from_str::<serde_json::Value>(&spec.body.text)
                .map_err(|e| format!("JSON 格式错误：{e}"))?;
            request.body(spec.body.text.clone())
        }
        "text" => request.body(spec.body.text.clone()),
        "form" => request.form(
            &spec
                .body
                .fields
                .iter()
                .filter(|p| p.enabled && !p.key.is_empty())
                .map(|p| (&p.key, &p.value))
                .collect::<Vec<_>>(),
        ),
        "multipart" => {
            let mut form = reqwest::multipart::Form::new();
            for field in spec
                .body
                .fields
                .iter()
                .filter(|p| p.enabled && !p.key.is_empty())
            {
                if field.kind == "file" {
                    let file = tokio::fs::File::open(&field.value)
                        .await
                        .map_err(|e| format!("文件不可读（{}）：{e}", field.value))?;
                    let metadata = file
                        .metadata()
                        .await
                        .map_err(|e| format!("文件不可读：{e}"))?;
                    if !metadata.is_file() {
                        return Err("上传路径必须是文件".into());
                    }
                    let filename = std::path::Path::new(&field.value)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    let body = reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(file));
                    form = form.part(
                        field.key.clone(),
                        reqwest::multipart::Part::stream_with_length(body, metadata.len())
                            .file_name(filename),
                    );
                } else {
                    form = form.text(field.key.clone(), field.value.clone());
                }
            }
            request.multipart(form)
        }
        _ => return Err("未知请求体类型".into()),
    };
    let mut response = request.send().await.map_err(network_error)?;
    let status = response.status().as_u16();
    let final_url = response.url().to_string();
    let headers = response
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.to_string(),
                String::from_utf8_lossy(v.as_bytes()).into_owned(),
            )
        })
        .collect();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let mut bytes = Vec::new();
    let mut truncated = false;
    while let Some(chunk) = response.chunk().await.map_err(network_error)? {
        let remaining = MAX_BODY - bytes.len();
        if chunk.len() > remaining {
            bytes.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        bytes.extend_from_slice(&chunk);
    }
    let binary = !(content_type.starts_with("text/")
        || content_type.contains("json")
        || content_type.contains("xml")
        || content_type.contains("javascript")
        || (content_type.is_empty() && !bytes.contains(&0) && std::str::from_utf8(&bytes).is_ok()));
    Ok(ResponseData {
        status,
        final_url,
        headers,
        elapsed_ms: started.elapsed().as_millis() as u64,
        size: bytes.len(),
        text: if binary {
            None
        } else {
            Some(String::from_utf8_lossy(&bytes).into_owned())
        },
        binary,
        truncated,
    })
}
fn network_error(error: reqwest::Error) -> String {
    use std::error::Error;
    let mut detail = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        detail.push_str(&format!("：{cause}"));
        source = cause.source();
    }
    let lower = detail.to_lowercase();
    if error.is_timeout() {
        "请求超时".into()
    } else if lower.contains("certificate") || lower.contains("tls") || lower.contains("ssl") {
        format!("TLS 证书或握手错误：{detail}")
    } else if error.is_connect() {
        format!("连接失败：{detail}")
    } else {
        format!("请求失败：{detail}")
    }
}
