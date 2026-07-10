use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use url::Url;

use super::{provider_error, ProviderResult};
use crate::domain::ProviderErrorKind;

const SESSION_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const MAX_REQUEST_BYTES: usize = 64 * 1024;
const MAX_MANIFEST_BYTES: u64 = 5 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;

#[derive(Debug)]
struct PlaybackSession {
    origin: String,
    headers: Vec<(String, String)>,
    routes: HashMap<String, String>,
    created_at: Instant,
}

static SESSIONS: OnceLock<Mutex<HashMap<String, PlaybackSession>>> = OnceLock::new();
static PROXY_PORT: OnceLock<Result<u16, String>> = OnceLock::new();

pub fn protect_hls_target(url: String, headers: Vec<(String, String)>) -> ProviderResult<String> {
    let parsed = validate_upstream_url(&url)?;
    let origin = parsed.origin().ascii_serialization();
    if origin == "null" {
        return Err(proxy_error("HLS origin is invalid"));
    }
    let port = ensure_proxy_started()?;
    let session_id = uuid::Uuid::new_v4().simple().to_string();
    let route_id = uuid::Uuid::new_v4().simple().to_string();
    let mut routes = HashMap::new();
    routes.insert(route_id.clone(), url);
    let session = PlaybackSession {
        origin,
        headers: sanitize_headers(headers),
        routes,
        created_at: Instant::now(),
    };
    let mut sessions = sessions()
        .lock()
        .map_err(|_| proxy_error("playback session storage is unavailable"))?;
    sessions.retain(|_, session| session.created_at.elapsed() < SESSION_TTL);
    sessions.insert(session_id.clone(), session);
    Ok(proxy_url(port, &session_id, &route_id))
}

fn sessions() -> &'static Mutex<HashMap<String, PlaybackSession>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ensure_proxy_started() -> ProviderResult<u16> {
    match PROXY_PORT.get_or_init(|| {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| "bind failed".to_string())?;
        let port = listener
            .local_addr()
            .map_err(|_| "local address unavailable".to_string())?
            .port();
        std::thread::Builder::new()
            .name("anime-provider-hls".to_string())
            .spawn(move || serve(listener))
            .map_err(|_| "thread start failed".to_string())?;
        Ok(port)
    }) {
        Ok(port) => Ok(*port),
        Err(_) => Err(proxy_error("secure HLS proxy could not be started")),
    }
}

fn serve(listener: TcpListener) {
    for connection in listener.incoming() {
        let Ok(stream) = connection else { continue };
        let _ = std::thread::Builder::new()
            .name("anime-provider-hls-request".to_string())
            .spawn(move || handle_connection(stream));
    }
}

fn handle_connection(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    let request = match read_request(&mut stream) {
        Ok(request) => request,
        Err(_) => {
            let _ = write_empty_response(&mut stream, 400, "Bad Request");
            return;
        }
    };
    let mut request_lines = request.lines();
    let Some(request_line) = request_lines.next() else {
        let _ = write_empty_response(&mut stream, 400, "Bad Request");
        return;
    };
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();
    if method == "OPTIONS" {
        let _ = write_options_response(&mut stream);
        return;
    }
    if !matches!(method, "GET" | "HEAD") {
        let _ = write_empty_response(&mut stream, 405, "Method Not Allowed");
        return;
    }
    let Some((session_id, route_id)) = parse_proxy_path(path) else {
        let _ = write_empty_response(&mut stream, 404, "Not Found");
        return;
    };
    let range = request_lines.find_map(|line| {
        line.split_once(':').and_then(|(name, value)| {
            name.eq_ignore_ascii_case("range")
                .then(|| value.trim().to_string())
        })
    });
    let (upstream_url, origin, headers) = match session_route(&session_id, &route_id) {
        Ok(value) => value,
        Err(_) => {
            let _ = write_empty_response(&mut stream, 404, "Not Found");
            return;
        }
    };
    let (response, final_url) =
        match fetch_upstream(&upstream_url, &origin, &headers, range.as_deref()) {
            Ok(value) => value,
            Err(_) => {
                let _ = write_empty_response(&mut stream, 502, "Bad Gateway");
                return;
            }
        };

    let status = response.status();
    let content_type = response
        .header("content-type")
        .unwrap_or("application/octet-stream")
        .to_string();
    let content_range = response.header("content-range").map(str::to_string);
    let accept_ranges = response.header("accept-ranges").map(str::to_string);
    let content_length = response
        .header("content-length")
        .and_then(|value| value.parse::<u64>().ok());
    let looks_like_manifest = content_type.to_ascii_lowercase().contains("mpegurl")
        || final_url
            .to_ascii_lowercase()
            .split('?')
            .next()
            .is_some_and(|value| value.ends_with(".m3u8"));

    if looks_like_manifest {
        let mut reader = response.into_reader().take(MAX_MANIFEST_BYTES + 1);
        let mut body = Vec::new();
        if reader.read_to_end(&mut body).is_err() || body.len() as u64 > MAX_MANIFEST_BYTES {
            let _ = write_empty_response(&mut stream, 413, "Payload Too Large");
            return;
        }
        let rewritten =
            match rewrite_manifest(&session_id, &final_url, &String::from_utf8_lossy(&body)) {
                Ok(body) => body.into_bytes(),
                Err(_) => {
                    let _ = write_empty_response(&mut stream, 502, "Bad Gateway");
                    return;
                }
            };
        let _ = write_headers(
            &mut stream,
            status,
            "application/vnd.apple.mpegurl",
            Some(rewritten.len() as u64),
            content_range.as_deref(),
            accept_ranges.as_deref(),
        );
        if method != "HEAD" {
            let _ = stream.write_all(&rewritten);
        }
        return;
    }

    let _ = write_headers(
        &mut stream,
        status,
        &content_type,
        content_length,
        content_range.as_deref(),
        accept_ranges.as_deref(),
    );
    if method != "HEAD" {
        let mut reader = response.into_reader();
        let _ = std::io::copy(&mut reader, &mut stream);
    }
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut bytes = Vec::with_capacity(4096);
    let mut buffer = [0u8; 2048];
    while bytes.len() < MAX_REQUEST_BYTES {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    if bytes.len() >= MAX_REQUEST_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "request too large",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid request"))
}

fn session_route(session_id: &str, route_id: &str) -> ProviderResult<UpstreamResponseParts> {
    let mut sessions = sessions()
        .lock()
        .map_err(|_| proxy_error("playback session storage is unavailable"))?;
    sessions.retain(|_, session| session.created_at.elapsed() < SESSION_TTL);
    let session = sessions
        .get(session_id)
        .ok_or_else(|| proxy_error("playback session expired"))?;
    let url = session
        .routes
        .get(route_id)
        .ok_or_else(|| proxy_error("playback route is unavailable"))?;
    Ok((url.clone(), session.origin.clone(), session.headers.clone()))
}

fn register_route(session_id: &str, url: String) -> ProviderResult<String> {
    let port = ensure_proxy_started()?;
    let mut sessions = sessions()
        .lock()
        .map_err(|_| proxy_error("playback session storage is unavailable"))?;
    let session = sessions
        .get_mut(session_id)
        .ok_or_else(|| proxy_error("playback session expired"))?;
    let parsed = validate_upstream_url(&url)?;
    if parsed.origin().ascii_serialization() != session.origin {
        return Err(proxy_error("cross-origin HLS route was rejected"));
    }
    if let Some((route_id, _)) = session
        .routes
        .iter()
        .find(|(_, existing)| *existing == &url)
    {
        return Ok(proxy_url(port, session_id, route_id));
    }
    let route_id = uuid::Uuid::new_v4().simple().to_string();
    session.routes.insert(route_id.clone(), url);
    Ok(proxy_url(port, session_id, &route_id))
}

fn rewrite_manifest(session_id: &str, base_url: &str, content: &str) -> ProviderResult<String> {
    let base = validate_upstream_url(base_url)?;
    let mut result = String::with_capacity(content.len() * 2);
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            result.push('\n');
        } else if trimmed.starts_with('#') {
            result.push_str(&rewrite_uri_attributes(session_id, &base, line)?);
            result.push('\n');
        } else {
            let absolute = base
                .join(trimmed)
                .map_err(|_| proxy_error("invalid HLS route"))?;
            result.push_str(&register_route(session_id, absolute.to_string())?);
            result.push('\n');
        }
    }
    Ok(result)
}

fn rewrite_uri_attributes(session_id: &str, base: &Url, line: &str) -> ProviderResult<String> {
    let mut output = String::with_capacity(line.len() * 2);
    let mut remaining = line;
    while let Some(start) = remaining.find("URI=") {
        output.push_str(&remaining[..start + 4]);
        remaining = &remaining[start + 4..];
        let Some(quote) = remaining
            .chars()
            .next()
            .filter(|character| matches!(character, '\'' | '"'))
        else {
            output.push_str(remaining);
            return Ok(output);
        };
        output.push(quote);
        remaining = &remaining[quote.len_utf8()..];
        let Some(end) = remaining.find(quote) else {
            output.push_str(remaining);
            return Ok(output);
        };
        let absolute = base
            .join(&remaining[..end])
            .map_err(|_| proxy_error("invalid HLS URI"))?;
        output.push_str(&register_route(session_id, absolute.to_string())?);
        output.push(quote);
        remaining = &remaining[end + quote.len_utf8()..];
    }
    output.push_str(remaining);
    Ok(output)
}

type UpstreamResponseParts = (String, String, Vec<(String, String)>);

fn fetch_upstream(
    initial_url: &str,
    origin: &str,
    headers: &[(String, String)],
    range: Option<&str>,
) -> Result<(ureq::Response, String), ()> {
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(Duration::from_secs(30))
        .build();
    let mut current = initial_url.to_string();
    for _ in 0..=MAX_REDIRECTS {
        let parsed = validate_upstream_url(&current).map_err(|_| ())?;
        if parsed.origin().ascii_serialization() != origin {
            return Err(());
        }
        let mut request = agent
            .get(&current)
            .set("Accept", "*/*")
            .set("Accept-Encoding", "identity")
            .set("User-Agent", "MoePlay Anime Provider/2");
        for (name, value) in headers {
            request = request.set(name, value);
        }
        if let Some(range) = range {
            request = request.set("Range", range);
        }
        let response = match request.call() {
            Ok(response) => response,
            Err(ureq::Error::Status(_, response)) => response,
            Err(ureq::Error::Transport(_)) => return Err(()),
        };
        if matches!(response.status(), 301 | 302 | 303 | 307 | 308) {
            let Some(location) = response.header("location") else {
                return Err(());
            };
            current = parsed.join(location).map_err(|_| ())?.to_string();
            continue;
        }
        let final_url = if response.get_url().is_empty() {
            current
        } else {
            response.get_url().to_string()
        };
        return Ok((response, final_url));
    }
    Err(())
}

fn validate_upstream_url(value: &str) -> ProviderResult<Url> {
    let parsed = Url::parse(value).map_err(|_| proxy_error("HLS URL is invalid"))?;
    let local_http = parsed.scheme() == "http"
        && parsed
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case("localhost"));
    if parsed.scheme() != "https" && !local_http {
        return Err(proxy_error("HLS URL must use HTTPS or localhost HTTP"));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() || parsed.fragment().is_some() {
        return Err(proxy_error(
            "HLS URL contains unsupported credentials or fragments",
        ));
    }
    Ok(parsed)
}

fn sanitize_headers(headers: Vec<(String, String)>) -> Vec<(String, String)> {
    headers
        .into_iter()
        .filter(|(name, value)| {
            !value.trim().is_empty()
                && matches!(
                    name.to_ascii_lowercase().as_str(),
                    "authorization" | "x-emby-token" | "x-mediabrowser-token"
                )
        })
        .collect()
}

fn parse_proxy_path(path: &str) -> Option<(String, String)> {
    let path = path.split('?').next()?;
    let mut parts = path.trim_start_matches('/').split('/');
    if parts.next()? != "anime-provider" {
        return None;
    }
    let session = parts.next()?.to_string();
    let route = parts.next()?.to_string();
    if session.len() != 32 || route.len() != 32 || parts.next().is_some() {
        return None;
    }
    Some((session, route))
}

fn proxy_url(port: u16, session_id: &str, route_id: &str) -> String {
    format!("http://127.0.0.1:{port}/anime-provider/{session_id}/{route_id}")
}

fn write_options_response(stream: &mut TcpStream) -> std::io::Result<()> {
    stream.write_all(b"HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, HEAD, OPTIONS\r\nAccess-Control-Allow-Headers: Range, Content-Type\r\nAccess-Control-Max-Age: 86400\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
}

fn write_empty_response(stream: &mut TcpStream, status: u16, text: &str) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status} {text}\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

fn write_headers(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    content_length: Option<u64>,
    content_range: Option<&str>,
    accept_ranges: Option<&str>,
) -> std::io::Result<()> {
    let text = match status {
        200 => "OK",
        206 => "Partial Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        416 => "Range Not Satisfiable",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Upstream Response",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {text}\r\nContent-Type: {content_type}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, HEAD, OPTIONS\r\nAccess-Control-Allow-Headers: Range, Content-Type\r\nAccess-Control-Expose-Headers: Content-Range, Content-Length, Content-Type, Accept-Ranges\r\nConnection: close\r\nCache-Control: no-store\r\n"
    )?;
    if let Some(length) = content_length {
        write!(stream, "Content-Length: {length}\r\n")?;
    }
    if let Some(range) = content_range {
        write!(stream, "Content-Range: {range}\r\n")?;
    }
    if let Some(ranges) = accept_ranges {
        write!(stream, "Accept-Ranges: {ranges}\r\n")?;
    }
    stream.write_all(b"\r\n")
}

fn proxy_error(message: &str) -> crate::domain::ProviderError {
    provider_error(
        "anime_provider",
        "resolve",
        ProviderErrorKind::PolicyBlocked,
        message,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_url_is_loopback_and_does_not_expose_headers_or_upstream() {
        let protected = protect_hls_target(
            "https://media.example/library/master.m3u8".to_string(),
            vec![("X-Emby-Token".to_string(), "private-token".to_string())],
        )
        .unwrap();
        assert!(protected.starts_with("http://127.0.0.1:"));
        assert!(!protected.contains("private-token"));
        assert!(!protected.contains("media.example"));
    }

    #[test]
    fn only_same_origin_routes_can_be_registered() {
        let protected = protect_hls_target(
            "https://media.example/library/master.m3u8".to_string(),
            vec![],
        )
        .unwrap();
        let (_, path) = protected.split_once("/anime-provider/").unwrap();
        let (session, _) = path.split_once('/').unwrap();
        assert!(register_route(
            session,
            "https://media.example/library/segment.ts".to_string()
        )
        .is_ok());
        assert!(register_route(session, "https://evil.example/segment.ts".to_string()).is_err());
    }
}
