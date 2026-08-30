//! Тесты сетевого слоя: разбор прокси-списков, автоопределение прокси
//! из окружения, понятные ошибки при живых запросах.

use renderex::fetch;
use std::sync::Mutex;

/// Защита от параллельного изменения переменных окружения.
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn proxy_list_wininet_format() {
    // "http=...;https=..." — https предпочтительнее.
    let p = fetch::pick_from_list("http=proxy1:8080;https=secure:8443");
    assert_eq!(p.as_deref(), Some("http://secure:8443"));
    // Только http.
    let p = fetch::pick_from_list("http=proxy1:8080");
    assert_eq!(p.as_deref(), Some("http://proxy1:8080"));
}

#[test]
fn proxy_list_pac_format() {
    let p = fetch::pick_from_list("PROXY 10.0.0.1:3128; PROXY 10.0.0.2:3128; DIRECT");
    assert_eq!(p.as_deref(), Some("http://10.0.0.1:3128"));
    let p = fetch::pick_from_list("DIRECT");
    assert_eq!(p, None);
    let p = fetch::pick_from_list("PROXY https://proxy.example:8443");
    assert_eq!(p.as_deref(), Some("https://proxy.example:8443"));
}

#[test]
fn proxy_list_plain() {
    let p = fetch::pick_from_list("192.168.1.1:8080");
    assert_eq!(p.as_deref(), Some("http://192.168.1.1:8080"));
    let p = fetch::pick_from_list("");
    assert_eq!(p, None);
}

#[test]
fn proxy_from_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("RENDREX_PROXY", "proxy.local:9999");
    let p = fetch::resolve_auto_proxy();
    std::env::remove_var("RENDREX_PROXY");
    assert_eq!(p.as_deref(), Some("http://proxy.local:9999"));
}

#[test]
fn proxy_normalize() {
    assert_eq!(fetch::normalize_proxy("h:1".into()), "http://h:1");
    assert_eq!(
        fetch::normalize_proxy("https://h:1".into()),
        "https://h:1"
    );
    assert_eq!(
        fetch::normalize_proxy("socks5://h:1".into()),
        "socks5://h:1"
    );
}

#[test]
fn html_page_gives_friendly_error() {
    // example.com отдаёт text/html — должен сработать понятный диагноз.
    let err = fetch::fetch_http("https://example.com").unwrap_err();
    assert!(
        err.contains("HTML-страница"),
        "ожидался диагноз про HTML, получено: {err}"
    );
}

#[test]
fn http_404_gives_friendly_error() {
    let err = fetch::fetch_http("https://example.com/renderex-not-found-xyz").unwrap_err();
    assert!(
        err.contains("404"),
        "ожидалась ошибка 404, получено: {err}"
    );
}

#[test]
fn bad_domain_gives_friendly_error() {
    let err = fetch::fetch_http("https://no-such-host-renderex-xyz.invalid/a.png").unwrap_err();
    assert!(
        err.contains("не удалось") && err.contains("прокси"),
        "ожидалась сетевая ошибка с подсказкой про прокси, получено: {err}"
    );
}

#[test]
fn url_cache_returns_same_bytes() {
    // Второй вызов должен попасть в кэш (и вернуть тот же результат).
    let a = fetch::fetch_http("https://picsum.photos/seed/renderex_cache/20/20").unwrap();
    let b = fetch::fetch_http("https://picsum.photos/seed/renderex_cache/20/20").unwrap();
    assert_eq!(a, b);
    assert!(!a.is_empty());
}
