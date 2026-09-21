use super::*;
use axum::{
    body::{to_bytes, Body},
    http::{Request, Response, StatusCode},
    Router,
};
use engine::{execute, MAX_BODY};
use std::time::Duration;
struct TestServer {
    url: String,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn server() -> TestServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, Router::new().fallback(handler))
            .await
            .unwrap();
    });
    TestServer { url, task }
}
async fn handler(req: Request<Body>) -> Response<Body> {
    match req.uri().path() {
        "/slow" => {
            tokio::time::sleep(Duration::from_millis(300)).await;
            Response::new(Body::from("slow"))
        }
        "/redirect" => Response::builder()
            .status(302)
            .header("location", "/echo")
            .body(Body::empty())
            .unwrap(),
        "/loop" => Response::builder()
            .status(302)
            .header("location", "/loop")
            .body(Body::empty())
            .unwrap(),
        "/binary" => Response::builder()
            .header("content-type", "application/octet-stream")
            .body(Body::from(vec![0, 1, 255]))
            .unwrap(),
        "/large" => Response::builder()
            .header("content-type", "text/plain")
            .body(Body::from(vec![b'x'; MAX_BODY + 1]))
            .unwrap(),
        "/cookies" => Response::builder()
            .header("set-cookie", "session=abc; Path=/; HttpOnly")
            .header("set-cookie", "saved=yes; Path=/; Max-Age=3600")
            .body(Body::from("ok"))
            .unwrap(),
        "/error" => Response::builder()
            .status(422)
            .header("content-type", "application/json")
            .body(Body::from("{\"error\":\"invalid\"}"))
            .unwrap(),
        "/html" => Response::builder()
            .header("content-type", "text/html")
            .body(Body::from("<script>alert(1)</script>"))
            .unwrap(),
        _ => {
            let (parts, body) = req.into_parts();
            let body = to_bytes(body, 20 * 1024 * 1024).await.unwrap();
            let json = serde_json::json!({"method":parts.method.as_str(),"query":parts.uri.query(),"body":String::from_utf8_lossy(&body),"authorization":parts.headers.get("authorization").map(|v|v.to_str().unwrap()),"contentType":parts.headers.get("content-type").map(|v|v.to_str().unwrap()),"cookie":parts.headers.get("cookie").map(|v|v.to_str().unwrap()),"headers":parts.headers.get_all("x-repeat").iter().map(|v|v.to_str().unwrap()).collect::<Vec<_>>()});
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .header("x-repeat", "one")
                .header("x-repeat", "two")
                .body(Body::from(json.to_string()))
                .unwrap()
        }
    }
}
fn spec(url: &str) -> RequestSpec {
    RequestSpec {
        id: "test".into(),
        name: "test".into(),
        method: "GET".into(),
        url: url.into(),
        params: vec![],
        headers: vec![],
        auth: Auth {
            kind: "none".into(),
            token: "".into(),
            username: "".into(),
            password: "".into(),
        },
        body: RequestBody {
            kind: "none".into(),
            text: "".into(),
            fields: vec![],
        },
        timeout_ms: 3000,
    }
}
fn field(key: &str, value: &str) -> Pair {
    Pair {
        id: key.into(),
        key: key.into(),
        value: value.into(),
        enabled: true,
        kind: "text".into(),
    }
}
async fn send(client: &reqwest::Client, request: &RequestSpec) -> ResponseData {
    execute(client, request, CancellationToken::new())
        .await
        .unwrap()
}
fn json(response: &ResponseData) -> serde_json::Value {
    serde_json::from_str(response.text.as_ref().unwrap()).unwrap()
}
#[tokio::test]
async fn methods_parameters_headers_and_http_errors() {
    let server = server().await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut request = spec(&format!("{}/echo?a=1&a=2", server.url));
    request.params = vec![field("a", "1"), field("a", "2")];
    request.headers = vec![field("x-repeat", "first"), field("x-repeat", "second")];
    for method in ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"] {
        request.method = method.into();
        let response = send(&client, &request).await;
        let body = json(&response);
        assert_eq!(body["method"], method);
        assert_eq!(body["query"], "a=1&a=2");
        assert_eq!(body["headers"], serde_json::json!(["first", "second"]));
        assert_eq!(
            response
                .headers
                .iter()
                .filter(|(k, _)| k == "x-repeat")
                .count(),
            2
        );
    }
    request.method = "HEAD".into();
    assert_eq!(send(&client, &request).await.size, 0);
    request.method = "GET".into();
    request.url = format!("{}/error", server.url);
    assert_eq!(send(&client, &request).await.status, 422);
}
#[tokio::test]
async fn bodies_and_auth() {
    let server = server().await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut request = spec(&format!("{}/echo", server.url));
    request.method = "POST".into();
    request.body.kind = "json".into();
    request.body.text = "{\"ok\":true}".into();
    request.auth.kind = "bearer".into();
    request.auth.token = "token".into();
    request.headers.push(field("Authorization", "wrong"));
    let body = json(&send(&client, &request).await);
    assert_eq!(body["authorization"], "Bearer token");
    assert_eq!(body["body"], request.body.text);
    assert_eq!(body["contentType"], "application/json");
    request.auth.kind = "basic".into();
    request.auth.username = "user".into();
    request.auth.password = "pass".into();
    request.body.kind = "text".into();
    request.body.text = "hello 世界".into();
    let body = json(&send(&client, &request).await);
    assert_eq!(body["authorization"], "Basic dXNlcjpwYXNz");
    assert_eq!(body["body"], "hello 世界");
    request.body.kind = "form".into();
    request.body.fields = vec![field("a", "1"), field("a", "2"), field("q", "a b")];
    request.body.fields.push(Pair {
        enabled: false,
        ..field("off", "no")
    });
    assert_eq!(
        json(&send(&client, &request).await)["body"],
        "a=1&a=2&q=a+b"
    );
    request.body.kind = "json".into();
    request.body.text = "{".into();
    assert!(execute(&client, &request, CancellationToken::new())
        .await
        .unwrap_err()
        .contains("JSON"));
}
#[tokio::test]
async fn multipart_streams_and_missing_files() {
    let server = server().await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("hello.txt");
    std::fs::write(&file, "upload-content").unwrap();
    let mut request = spec(&format!("{}/echo", server.url));
    request.method = "POST".into();
    request.body.kind = "multipart".into();
    request.headers.push(field("Content-Type", "wrong"));
    request.body.fields = vec![
        field("name", "demo"),
        Pair {
            kind: "file".into(),
            ..field("upload", file.to_str().unwrap())
        },
    ];
    let body = json(&send(&client, &request).await);
    assert!(body["contentType"]
        .as_str()
        .unwrap()
        .starts_with("multipart/form-data; boundary="));
    let raw = body["body"].as_str().unwrap();
    assert!(raw.contains("upload-content"));
    assert!(raw.contains("hello.txt"));
    assert!(raw.contains("demo"));
    request.body.fields[1].value = dir.path().join("missing").to_string_lossy().into_owned();
    assert!(execute(&client, &request, CancellationToken::new())
        .await
        .unwrap_err()
        .contains("文件不可读"));
}
#[tokio::test]
async fn timeout_cancel_and_retry() {
    let server = server().await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut request = spec(&format!("{}/slow", server.url));
    request.timeout_ms = 20;
    assert_eq!(
        execute(&client, &request, CancellationToken::new())
            .await
            .unwrap_err(),
        "请求超时"
    );
    request.timeout_ms = 3000;
    let token = CancellationToken::new();
    let cancellation = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancellation.cancel();
    });
    assert_eq!(
        execute(&client, &request, token).await.unwrap_err(),
        "请求已取消"
    );
    request.url = format!("{}/echo", server.url);
    assert_eq!(send(&client, &request).await.status, 200);
}
#[tokio::test]
async fn redirects_binary_html_and_limit() {
    let server = server().await;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .unwrap();
    let response = send(&client, &spec(&format!("{}/redirect", server.url))).await;
    assert_eq!(response.final_url, format!("{}/echo", server.url));
    assert!(execute(
        &client,
        &spec(&format!("{}/loop", server.url)),
        CancellationToken::new()
    )
    .await
    .is_err());
    let response = send(&client, &spec(&format!("{}/binary", server.url))).await;
    assert!(response.binary);
    assert!(response.text.is_none());
    assert_eq!(response.size, 3);
    assert!(send(&client, &spec(&format!("{}/html", server.url)))
        .await
        .text
        .unwrap()
        .contains("<script>"));
    let response = send(&client, &spec(&format!("{}/large", server.url))).await;
    assert!(response.truncated);
    assert_eq!(response.size, MAX_BODY);
}
#[tokio::test]
async fn cookies_roundtrip_session_persistence_and_delete() {
    let server = server().await;
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::new(dir.path().to_path_buf()).unwrap();
    let response = send(&state.client, &spec(&format!("{}/cookies", server.url))).await;
    assert_eq!(
        response
            .headers
            .iter()
            .filter(|(k, _)| k == "set-cookie")
            .count(),
        2
    );
    let response = json(&send(&state.client, &spec(&format!("{}/echo", server.url))).await);
    let cookies = response["cookie"].as_str().unwrap();
    assert!(cookies.contains("session=abc"));
    assert!(cookies.contains("saved=yes"));
    state.save_cookies().unwrap();
    let restarted = AppState::new(dir.path().to_path_buf()).unwrap();
    let response = json(&send(&restarted.client, &spec(&format!("{}/echo", server.url))).await);
    let cookies = response["cookie"].as_str().unwrap();
    assert!(!cookies.contains("session"));
    assert!(cookies.contains("saved=yes"));
    restarted
        .jar
        .lock()
        .unwrap()
        .remove("127.0.0.1", "/", "saved");
    restarted.save_cookies().unwrap();
    let restarted = AppState::new(dir.path().to_path_buf()).unwrap();
    assert_eq!(restarted.jar.lock().unwrap().iter_unexpired().count(), 0);
}
#[test]
fn cookie_scope_expiry_and_clear() {
    let mut jar = cookie_store::CookieStore::default();
    let url = reqwest::Url::parse("https://example.com/api/set").unwrap();
    for cookie in [
        "host=a; Path=/api",
        "wide=b; Domain=example.com; Path=/",
        "secure=c; Secure; Path=/",
        "expired=d; Max-Age=0; Path=/",
    ] {
        let _ = jar.parse(cookie, &url);
    }
    let values = |jar: &cookie_store::CookieStore, url: &str| {
        jar.get_request_values(&reqwest::Url::parse(url).unwrap())
            .map(|(k, _)| k.to_string())
            .collect::<Vec<_>>()
    };
    assert!(values(&jar, "https://example.com/api/get").contains(&"host".into()));
    assert!(!values(&jar, "https://example.com/else").contains(&"host".into()));
    assert!(!values(&jar, "https://sub.example.com/api/get").contains(&"host".into()));
    assert!(values(&jar, "https://sub.example.com/").contains(&"wide".into()));
    assert!(!values(&jar, "http://example.com/").contains(&"secure".into()));
    assert!(!values(&jar, "https://example.com/").contains(&"expired".into()));
    jar.clear();
    assert_eq!(jar.iter_unexpired().count(), 0);
}
#[test]
fn workspace_roundtrip_and_corruption_protection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workspace.json");
    let mut data = SavedData::default();
    data.favorites.push(spec("http://localhost/"));
    data.history.push(History {
        id: "h".into(),
        at: 1,
        request: spec("http://localhost/"),
        status: Some(200),
        elapsed_ms: 20,
        error: None,
    });
    storage::save(&path, &data).unwrap();
    let loaded = storage::load(&path).unwrap();
    assert_eq!(loaded.favorites.len(), 1);
    assert_eq!(loaded.history.len(), 1);
    let serialized = std::fs::read_to_string(&path).unwrap();
    assert!(!serialized.contains("response"));
    std::fs::write(&path, "broken").unwrap();
    assert!(storage::load(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken");
}
#[tokio::test]
async fn invalid_url_header_and_connection() {
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    for url in ["not a url", "file:///etc/hosts"] {
        assert!(execute(&client, &spec(url), CancellationToken::new())
            .await
            .unwrap_err()
            .contains("URL"));
    }
    let mut request = spec("http://127.0.0.1:1");
    assert!(execute(&client, &request, CancellationToken::new())
        .await
        .unwrap_err()
        .contains("连接失败"));
    request.headers.push(field("bad\nheader", "x"));
    assert!(execute(&client, &request, CancellationToken::new())
        .await
        .unwrap_err()
        .contains("请求头"));
}

#[test]
fn history_keeps_latest_200_without_dropping_favorites() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workspace.json");
    let mut data = SavedData::default();
    data.favorites.push(spec("https://example.com/saved"));
    for i in 0..200 {
        data.history.push(History {
            id: i.to_string(),
            at: i,
            request: spec("http://localhost/"),
            status: Some(200),
            elapsed_ms: 1,
            error: None,
        });
    }
    storage::save(&path, &data).unwrap();
    storage::record_history(
        &path,
        History {
            id: "latest".into(),
            at: 999,
            request: spec("http://localhost/latest"),
            status: None,
            elapsed_ms: 30,
            error: Some("请求已取消".into()),
        },
    )
    .unwrap();
    let data = storage::load(&path).unwrap();
    assert_eq!(data.history.len(), 200);
    assert_eq!(data.history[0].id, "latest");
    assert_eq!(data.history.last().unwrap().id, "198");
    assert_eq!(data.favorites[0].url, "https://example.com/saved");
}
