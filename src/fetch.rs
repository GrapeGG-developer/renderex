//! Загрузка изображений по HTTP(S) с умным определением прокси и
//! понятными ошибками на русском.
//!
//! Порядок определения прокси:
//!   1. флаг `--proxy <url|none>` (явное указание);
//!   2. переменная окружения `RENDREX_PROXY`;
//!   3. стандартные `HTTPS_PROXY` / `HTTP_PROXY` / `ALL_PROXY`;
//!   4. системный прокси Windows (настройки IE/WinINET, включая PAC);
//!   5. прямое соединение.
//!
//! Скачанные файлы кэшируются в памяти по URL, действует лимит размера.

use std::io::Read;
use std::sync::{Mutex, OnceLock};

/// Максимальный размер скачиваемого файла (50 МБ).
pub const MAX_BYTES: usize = 50 * 1024 * 1024;

static CACHE: OnceLock<Mutex<std::collections::HashMap<String, Vec<u8>>>> = OnceLock::new();
static PROXY_OVERRIDE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static RESOLVED_PROXY: OnceLock<Option<String>> = OnceLock::new();
static AGENT: OnceLock<Option<ureq::Agent>> = OnceLock::new();

/// Явно задать прокси из CLI. `"none"`/`"direct"` отключает прокси
/// полностью (включая системный). Вызывается в main до загрузки файлов.
pub fn set_proxy_override(proxy: Option<String>) {
    let cell = PROXY_OVERRIDE.get_or_init(|| Mutex::new(None));
    *cell.lock().unwrap() = proxy;
}

/// Прокси, действующий для этого процесса (с кэшированием).
fn effective_proxy() -> Option<String> {
    // Явное указание "--proxy none" отключает и автоопределение.
    let g = PROXY_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    match g.as_ref() {
        Some(s) if s.eq_ignore_ascii_case("none") || s.eq_ignore_ascii_case("direct") => None,
        Some(s) => Some(s.clone()),
        None => RESOLVED_PROXY.get_or_init(resolve_auto_proxy).clone(),
    }
}

/// Автоопределение прокси: env → системные настройки Windows.
/// Низкоуровневая функция: обычно вызывается автоматически при первой
/// загрузке файла.
pub fn resolve_auto_proxy() -> Option<String> {
    // 1) Специальная переменная renderex.
    if let Some(p) = env_nonempty("RENDREX_PROXY") {
        return Some(normalize_proxy(p));
    }
    // 2) Стандартные переменные.
    for var in ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
        if let Some(p) = env_nonempty(var) {
            return Some(normalize_proxy(p));
        }
    }
    // 3) Системный прокси Windows (IE/WinINET + PAC).
    #[cfg(windows)]
    {
        if let Some(p) = winproxy::system_proxy() {
            return Some(p);
        }
    }
    None
}

fn env_nonempty(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|s| !s.trim().is_empty())
}

/// `host:port` → `http://host:port`; `http://...` остаётся как есть.
pub fn normalize_proxy(p: String) -> String {
    let p = p.trim().to_string();
    if p.contains("://") {
        p
    } else {
        format!("http://{p}")
    }
}

fn agent() -> Result<&'static ureq::Agent, String> {
    let a = AGENT.get_or_init(|| {
        let mut builder = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .timeout_connect(std::time::Duration::from_secs(15))
            // Часть «блокировок» — это файрволы, которые рвут незнакомые
            // соединения; представить браузер никогда не помешает.
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Renderex/0.3");
        if let Some(p) = effective_proxy() {
            match ureq::Proxy::new(&p) {
                Ok(proxy) => builder = builder.proxy(proxy),
                Err(e) => {
                    eprintln!("renderex: предупреждение: не удалось применить прокси '{p}': {e}");
                }
            }
        }
        Some(builder.build())
    });
    a.as_ref()
        .ok_or_else(|| "не удалось создать HTTP-клиент".to_string())
}

/// Скачивает `url` (http/https) с кэшем по URL, повтором при сбое
/// транспорта и понятным описанием ошибки.
pub fn fetch_http(url: &str) -> Result<Vec<u8>, String> {
    if let Some(hit) = CACHE
        .get_or_init(|| Mutex::new(Default::default()))
        .lock()
        .unwrap()
        .get(url)
    {
        return Ok(hit.clone());
    }

    let agent = agent()?;
    let mut last_err = String::new();

    // Две попытки: при обрыве соединения повтор часто помогает.
    for _ in 0..2 {
        match agent.get(url).call() {
            Ok(resp) => {
                if let Some(ct) = resp.header("content-type") {
                    let ct = ct.to_ascii_lowercase();
                    if ct.contains("text/html") || ct.contains("application/xhtml") {
                        return Err(html_page_error(url));
                    }
                }

                let mut buf = Vec::new();
                let mut limited = resp.into_reader().take(MAX_BYTES as u64 + 1);
                if let Err(e) = limited.read_to_end(&mut buf) {
                    last_err = format!("ошибка чтения ответа с '{url}': {e}");
                    continue;
                }
                if buf.len() > MAX_BYTES {
                    return Err(format!(
                        "файл по ссылке '{url}' слишком большой (больше {} МБ)",
                        MAX_BYTES / 1024 / 1024
                    ));
                }

                CACHE
                    .get_or_init(|| Mutex::new(Default::default()))
                    .lock()
                    .unwrap()
                    .insert(url.to_string(), buf.clone());
                return Ok(buf);
            }
            Err(ureq::Error::Status(code, _resp)) => {
                return Err(status_error(code, url));
            }
            Err(ureq::Error::Transport(t)) => {
                last_err = transport_error(&t, url);
            }
        }
    }

    Err(last_err)
}

/// Ошибка «по ссылке пришла HTML-страница» — самый частый случай
/// «блокировок»: сервер/файрвол отдаёт страницу вместо картинки.
fn html_page_error(url: &str) -> String {
    format!(
        "по ссылке '{url}' пришла HTML-страница, а не изображение.\n\
         Обычно это значит, что ссылка ведёт на веб-страницу, а не на файл,\n\
         либо запрос перехвачен (блокировка, файрвол, портал с авторизацией).\n\
         Используйте прямую ссылку на файл картинки (.png, .jpg, .webp и т.п.)."
    )
}

fn status_error(code: u16, url: &str) -> String {
    let reason = match code {
        400 => "неверный запрос",
        401 | 403 => "доступ запрещён (нужна авторизация или сервер блокирует запрос)",
        404 => "файл не найден — проверьте, что ссылка ведёт прямо на картинку",
        410 => "файл был удалён с сервера",
        429 => "слишком много запросов, попробуйте позже",
        500..=599 => "ошибка на стороне сервера",
        _ => "сервер вернул ошибку",
    };
    format!("не удалось скачать '{url}': HTTP {code} — {reason}")
}

fn transport_error(t: &ureq::Transport, url: &str) -> String {
    use ureq::ErrorKind;
    // В ureq 2.12 таймауты приходят как Io с текстом "timed out".
    let msg = t.message().unwrap_or_default().to_ascii_lowercase();
    let timed_out = msg.contains("timed out") || msg.contains("timeout");
    match t.kind() {
        ErrorKind::Dns => format!(
            "не удалось разрешить домен в ссылке '{url}'.\n\
             Проверьте интернет-соединение, DNS и прокси."
        ),
        ErrorKind::ConnectionFailed => format!(
            "не удалось соединиться с сервером по ссылке '{url}'.\n\
             Проверьте интернет, прокси и файрвол."
        ),
        ErrorKind::InvalidProxyUrl => format!(
            "неверный адрес прокси при загрузке '{url}'. Проверьте параметр --proxy."
        ),
        ErrorKind::ProxyConnect | ErrorKind::ProxyUnauthorized => format!(
            "не удалось соединиться с прокси-сервером при загрузке '{url}'.\n\
             Проверьте адрес прокси (--proxy) и учётные данные."
        ),
        ErrorKind::Io if timed_out => format!(
            "истекло время ожидания при загрузке '{url}'.\n\
             Сервер не отвечает или соединение слишком медленное."
        ),
        _ => format!("сетевая ошибка при загрузке '{url}': {t}"),
    }
}

// ---------------------------------------------------------------------------
// Системный прокси Windows: WinHTTP/WinINET + PAC-скрипты.
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod winproxy {
    use super::pick_from_list;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::Networking::WinHttp::{
        WinHttpCloseHandle, WinHttpGetIEProxyConfigForCurrentUser, WinHttpGetProxyForUrl,
        WinHttpOpen, WINHTTP_AUTOPROXY_AUTO_DETECT, WINHTTP_AUTOPROXY_CONFIG_URL,
        WINHTTP_AUTO_DETECT_TYPE_DHCP, WINHTTP_AUTO_DETECT_TYPE_DNS_A,
        WINHTTP_AUTOPROXY_OPTIONS, WINHTTP_CURRENT_USER_IE_PROXY_CONFIG, WINHTTP_PROXY_INFO,
    };

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    unsafe fn read_wide(ptr: *const u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }

    unsafe fn free_wide(p: *mut u16) {
        if !p.is_null() {
            let _ = GlobalFree(p as _);
        }
    }

    /// Прокси из настроек IE/WinINET текущего пользователя.
    pub fn system_proxy() -> Option<String> {
        unsafe {
            let mut cfg = WINHTTP_CURRENT_USER_IE_PROXY_CONFIG::default();
            if WinHttpGetIEProxyConfigForCurrentUser(&mut cfg) == 0 {
                return None;
            }

            // Явный прокси-сервер: "host:port" или "http=...;https=...".
            if !cfg.lpszProxy.is_null() {
                let list = read_wide(cfg.lpszProxy);
                free_wide(cfg.lpszProxy);
                free_wide(cfg.lpszProxyBypass);
                free_wide(cfg.lpszAutoConfigUrl);
                return pick_from_list(&list);
            }

            // PAC-скрипт или автоопределение.
            if !cfg.lpszAutoConfigUrl.is_null() || cfg.fAutoDetect != 0 {
                let pac_url = read_wide(cfg.lpszAutoConfigUrl);
                let auto_detect = cfg.fAutoDetect != 0;
                free_wide(cfg.lpszAutoConfigUrl);
                free_wide(cfg.lpszProxyBypass);
                return resolve_pac(&pac_url, auto_detect);
            }
            None
        }
    }

    unsafe fn resolve_pac(pac_url: &str, auto_detect: bool) -> Option<String> {
        let url = to_wide("https://picsum.photos/seed/renderex/1/1");
        if !pac_url.is_empty() {
            let pac = to_wide(pac_url);
            let mut opts = WINHTTP_AUTOPROXY_OPTIONS::default();
            opts.dwFlags = WINHTTP_AUTOPROXY_CONFIG_URL;
            opts.lpszAutoConfigUrl = pac.as_ptr();
            opts.fAutoLogonIfChallenged = 1;
            let session = WinHttpOpen(std::ptr::null(), 0, std::ptr::null(), std::ptr::null(), 0);
            let mut info = WINHTTP_PROXY_INFO::default();
            let ok = WinHttpGetProxyForUrl(session, url.as_ptr(), &mut opts, &mut info);
            let result = if ok != 0 && !info.lpszProxy.is_null() {
                let list = read_wide(info.lpszProxy);
                pick_from_list(&list)
            } else {
                None
            };
            free_wide(info.lpszProxy);
            free_wide(info.lpszProxyBypass);
            let _ = WinHttpCloseHandle(session);
            if result.is_some() {
                return result;
            }
        }
        if auto_detect {
            let mut opts = WINHTTP_AUTOPROXY_OPTIONS::default();
            opts.dwFlags = WINHTTP_AUTOPROXY_AUTO_DETECT;
            opts.dwAutoDetectFlags = WINHTTP_AUTO_DETECT_TYPE_DHCP | WINHTTP_AUTO_DETECT_TYPE_DNS_A;
            opts.fAutoLogonIfChallenged = 1;
            let session = WinHttpOpen(std::ptr::null(), 0, std::ptr::null(), std::ptr::null(), 0);
            let mut info = WINHTTP_PROXY_INFO::default();
            let ok = WinHttpGetProxyForUrl(session, url.as_ptr(), &mut opts, &mut info);
            let result = if ok != 0 && !info.lpszProxy.is_null() {
                let list = read_wide(info.lpszProxy);
                pick_from_list(&list)
            } else {
                None
            };
            free_wide(info.lpszProxy);
            free_wide(info.lpszProxyBypass);
            let _ = WinHttpCloseHandle(session);
            return result;
        }
        None
    }
}

/// Разбор списка прокси в формате WinINET/PAC:
/// `"http=proxy1:8080;https=proxy2:8080"` или
/// `"PROXY host:port; PROXY host2:port2; DIRECT"` или просто `"host:port"`.
pub fn pick_from_list(list: &str) -> Option<String> {
    let mut fallback: Option<String> = None;
    for part in list.split(';') {
        let part = part.trim();
        if part.is_empty() || part.eq_ignore_ascii_case("direct") {
            continue;
        }
        let mut addr: Option<&str> = None;
        let lower = part.to_ascii_lowercase();
        if let Some(rest) = lower
            .strip_prefix("proxy ")
            .or_else(|| lower.strip_prefix("http "))
            .or_else(|| lower.strip_prefix("https "))
            .or_else(|| lower.strip_prefix("socks "))
        {
            // Схему берём из начала исходной строки (смещение одинаковое,
            // т.к. prefix искался по нижнему регистру той же длины).
            addr = Some(&part["proxy ".len()..]);
            if rest.contains("://") {
                return Some(part["proxy ".len()..].trim().to_string());
            }
        } else if let Some((scheme, rest)) = part.split_once('=') {
            // "https=host:port" — https предпочтительнее.
            if scheme.eq_ignore_ascii_case("https") && !rest.trim().is_empty() {
                return Some(crate::fetch::normalize_proxy(rest.trim().to_string()));
            }
            addr = Some(rest.trim());
        } else if part.contains(':') {
            addr = Some(part);
        }
        if let Some(a) = addr {
            let a = a.trim();
            if !a.is_empty() && fallback.is_none() {
                fallback = Some(crate::fetch::normalize_proxy(a.to_string()));
            }
        }
    }
    fallback
}
