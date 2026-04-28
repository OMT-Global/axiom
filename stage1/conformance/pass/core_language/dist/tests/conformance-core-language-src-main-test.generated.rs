#[allow(unused_imports)]
use std::collections::HashMap;
use std::panic;
use std::sync::Once;

const AXIOM_PACKAGE_ROOT: &str = "/Users/johnteneyckjr./src/axiom/stage1/conformance/pass/core_language";
const AXIOM_FS_ROOT: &str = "/Users/johnteneyckjr./src/axiom/stage1/conformance/pass/core_language";
const AXIOM_ENV_UNRESTRICTED: bool = false;
const AXIOM_ENV_ALLOWLIST: &[&str] = &[
];
const AXIOM_MAX_FS_READ_BYTES: u64 = 64 * 1024 * 1024;

struct AxiomRuntimeAbort;

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
struct AxiomTask<T> {
    value: T,
    canceled: bool,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
struct AxiomJoinHandle<T> {
    task: AxiomTask<T>,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
struct AxiomChannel<T> {
    slot: Option<T>,
    closed: bool,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
struct AxiomSelectResult<T> {
    selected: i64,
    value: Option<T>,
}

fn axiom_install_panic_hook() {
    static AXIOM_PANIC_HOOK: Once = Once::new();
    AXIOM_PANIC_HOOK.call_once(|| {
        panic::set_hook(Box::new(|_| {}));
    });
}

fn axiom_runtime_report(kind: &str, message: &str) {
    eprintln!(
        "{{\"kind\":\"{}\",\"message\":{}}}",
        kind,
        axiom_json_escape_string(message)
    );
}

fn axiom_runtime_error(kind: &str, message: &str) -> ! {
    axiom_runtime_report(kind, message);
    panic::panic_any(AxiomRuntimeAbort)
}

#[allow(dead_code)]
fn axiom_task_ready<T>(value: T) -> AxiomTask<T> {
    AxiomTask { value, canceled: false }
}

#[allow(dead_code)]
fn axiom_await<T>(task: AxiomTask<T>) -> T {
    if task.canceled {
        axiom_runtime_error("async", "awaited task was canceled");
    }
    task.value
}

#[allow(dead_code)]
fn axiom_array_get<T: Copy>(values: &[T], index: i64) -> T {
    if index < 0 {
        axiom_runtime_error("runtime", "array index must be non-negative");
    }
    match values.get(index as usize) {
        Some(value) => *value,
        None => axiom_runtime_error("runtime", "array index out of bounds"),
    }
}

#[allow(dead_code)]
fn axiom_array_take<T>(values: Vec<T>, index: i64) -> T {
    if index < 0 {
        axiom_runtime_error("runtime", "array index must be non-negative");
    }
    match values.into_iter().nth(index as usize) {
        Some(value) => value,
        None => axiom_runtime_error("runtime", "array index out of bounds"),
    }
}

#[allow(dead_code)]
fn axiom_array_slice_bounds(len: usize, start: Option<i64>, end: Option<i64>) -> (usize, usize) {
    let start = start.unwrap_or(0);
    let end = end.unwrap_or(len as i64);
    if start < 0 {
        axiom_runtime_error("runtime", "array slice start must be non-negative");
    }
    if end < 0 {
        axiom_runtime_error("runtime", "array slice end must be non-negative");
    }
    let start = start as usize;
    let end = end as usize;
    if start > end {
        axiom_runtime_error("runtime", "array slice start must be <= end");
    }
    if end > len {
        axiom_runtime_error("runtime", "array slice end out of bounds");
    }
    (start, end)
}

#[allow(dead_code)]
fn axiom_slice_view<'a, T>(values: &'a [T], start: Option<i64>, end: Option<i64>) -> &'a [T] {
    let (start, end) = axiom_array_slice_bounds(values.len(), start, end);
    match values.get(start..end) {
        Some(slice) => slice,
        None => axiom_runtime_error("runtime", "array slice out of bounds"),
    }
}

#[allow(dead_code)]
fn axiom_slice_view_mut<'a, T>(values: &'a mut [T], start: Option<i64>, end: Option<i64>) -> &'a mut [T] {
    let (start, end) = axiom_array_slice_bounds(values.len(), start, end);
    match values.get_mut(start..end) {
        Some(slice) => slice,
        None => axiom_runtime_error("runtime", "array slice out of bounds"),
    }
}

#[allow(dead_code)]
fn axiom_last_index(len: usize) -> i64 {
    if len == 0 {
        axiom_runtime_error("runtime", "collection must not be empty");
    }
    (len - 1) as i64
}

#[allow(dead_code)]
fn axiom_io_eprintln(text: String) -> i64 {
    use std::io::Write;
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    match handle.write_all(text.as_bytes()).and_then(|_| handle.write_all(b"\n")) {
        Ok(()) => (text.len() as i64) + 1,
        Err(_) => -1,
    }
}

#[allow(dead_code)]
fn axiom_assert_fail(message: String, _line: i64, _column: i64) -> i64 {
    axiom_runtime_error("assertion", &message)
}

#[allow(dead_code)]
fn axiom_json_parse_int(text: String) -> Option<i64> {
    text.trim().parse::<i64>().ok()
}

#[allow(dead_code)]
fn axiom_json_parse_bool(text: String) -> Option<bool> {
    match text.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[allow(dead_code)]
fn axiom_json_parse_string(text: String) -> Option<String> {
    let text = text.trim();
    if text.len() < 2 || !text.starts_with('"') || !text.ends_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = text[1..text.len() - 1].chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next()? {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            '/' => out.push('/'),
            'b' => out.push('\u{0008}'),
            'f' => out.push('\u{000C}'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'u' => {
                let mut value = 0u32;
                for _ in 0..4 {
                    value = (value << 4) + chars.next()?.to_digit(16)?;
                }
                out.push(char::from_u32(value)?);
            }
            _ => return None,
        }
    }
    Some(out)
}

#[allow(dead_code)]
fn axiom_json_escape_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[allow(dead_code)]
fn axiom_json_stringify_int(value: i64) -> String {
    value.to_string()
}

#[allow(dead_code)]
fn axiom_json_stringify_bool(value: bool) -> String {
    value.to_string()
}

#[allow(dead_code)]
fn axiom_json_stringify_string(value: String) -> String {
    axiom_json_escape_string(&value)
}

#[allow(dead_code)]
fn axiom_fs_read(path: String) -> Option<String> {
    use std::io::Read;
    let canonical_package_root = std::fs::canonicalize(AXIOM_PACKAGE_ROOT).ok()?;
    let canonical_fs_root = std::fs::canonicalize(AXIOM_FS_ROOT).ok()?;
    if !canonical_fs_root.starts_with(&canonical_package_root) {
        return None;
    }
    let requested = std::path::Path::new(&path);
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        canonical_package_root.join(requested)
    };
    let canonical_candidate = std::fs::canonicalize(candidate).ok()?;
    if !canonical_candidate.starts_with(&canonical_fs_root) {
        return None;
    }
    let metadata = std::fs::metadata(&canonical_candidate).ok()?;
    if !metadata.is_file() || metadata.len() > AXIOM_MAX_FS_READ_BYTES {
        return None;
    }
    let file = std::fs::File::open(&canonical_candidate).ok()?;
    let mut reader = file.take(AXIOM_MAX_FS_READ_BYTES + 1);
    let mut content = String::new();
    reader.read_to_string(&mut content).ok()?;
    if content.len() as u64 > AXIOM_MAX_FS_READ_BYTES {
        return None;
    }
    Some(content)
}

#[allow(dead_code)]
fn axiom_is_blocked_network_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(addr) => {
            let octets = addr.octets();
            addr.is_private()
                || addr.is_loopback()
                || addr.is_link_local()
                || addr.is_unspecified()
                || addr.is_broadcast()
                || addr.is_multicast()
                || octets[0] == 0
                || (octets[0] == 100 && (64..=127).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
                || (octets[0] == 198 && (18..=19).contains(&octets[1]))
                || (octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
                || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
        }
        std::net::IpAddr::V6(addr) => {
            if let Some(mapped) = addr.to_ipv4_mapped() {
                return axiom_is_blocked_network_ip(std::net::IpAddr::V4(mapped));
            }
            let segments = addr.segments();
            addr.is_loopback()
                || addr.is_unspecified()
                || addr.is_multicast()
                || (segments[0] & 0xfe00) == 0xfc00
                || (segments[0] & 0xffc0) == 0xfe80
                || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        }
    }
}

#[allow(dead_code)]
fn axiom_resolve_public_socket_addrs(host: &str, port: u16) -> Option<Vec<std::net::SocketAddr>> {
    use std::net::ToSocketAddrs;
    let addrs: Vec<std::net::SocketAddr> = (host, port).to_socket_addrs().ok()?.collect();
    if addrs.is_empty() {
        return None;
    }
    // Network intrinsics reject private, loopback, link-local,
    // multicast, documentation, and metadata-style addresses.
    if addrs.iter().any(|addr| axiom_is_blocked_network_ip(addr.ip())) {
        return None;
    }
    Some(addrs)
}

#[allow(dead_code)]
fn axiom_net_resolve(host: String) -> Option<String> {
    axiom_resolve_public_socket_addrs(host.as_str(), 0)?
        .into_iter()
        .next()
        .map(|addr| addr.ip().to_string())
}

#[allow(dead_code)]
fn axiom_process_status(program: String) -> i64 {
    std::process::Command::new(program)
        .status()
        .ok()
        .and_then(|status| status.code())
        .map(i64::from)
        .unwrap_or(-1)
}

#[allow(dead_code)]
fn axiom_clock_now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(now) => now,
        Err(_) => axiom_runtime_error("runtime", "system clock must be after unix epoch"),
    };
    now.as_millis() as i64
}

#[allow(dead_code)]
fn axiom_env_get(name: String) -> Option<String> {
    if !AXIOM_ENV_UNRESTRICTED && !AXIOM_ENV_ALLOWLIST.contains(&name.as_str()) {
        return None;
    }
    std::env::var(name).ok()
}

#[allow(dead_code)]
fn axiom_crypto_sha256(input: String) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut data = input.into_bytes();
    let bit_len = (data.len() as u64) * 8;
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in data.chunks(64) {
        let mut schedule = [0u32; 64];
        for (index, word) in schedule.iter_mut().take(16).enumerate() {
            let start = index * 4;
            *word = u32::from_be_bytes([
                chunk[start],
                chunk[start + 1],
                chunk[start + 2],
                chunk[start + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }
        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];
        for index in 0..64 {
            let sigma1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sigma1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(schedule[index]);
            let sigma0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sigma0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
    let mut output = String::new();
    for value in state {
        output.push_str(&format!("{value:08x}"));
    }
    output
}

#[allow(dead_code)]
fn axiom_map_get<K: Eq + std::hash::Hash, V: Copy>(values: &HashMap<K, V>, key: &K) -> V {
    match values.get(key) {
        Some(value) => *value,
        None => axiom_runtime_error("runtime", "map key not found"),
    }
}

#[allow(dead_code)]
fn axiom_map_take<K: Eq + std::hash::Hash, V>(mut values: HashMap<K, V>, key: &K) -> V {
    match values.remove(key) {
        Some(value) => value,
        None => axiom_runtime_error("runtime", "map key not found"),
    }
}

#[allow(non_snake_case)]
fn conformance_core_language_main_test_add(left: i64, right: i64) -> i64 {
    return left + right;
}

#[allow(non_snake_case)]
fn conformance_core_language_main_test_banner(name: String) -> String {
    return format!("{}{}", String::from("hello "), name);
}

fn main() -> std::process::ExitCode {
    axiom_install_panic_hook();
    let result = panic::catch_unwind(|| {
        let answer: i64 = conformance_core_language_main_test_add(40, 2);
        let ready: bool = answer == 42;
        println!("{}", answer);
        println!("{}", ready);
        if ready {
            println!("{}", conformance_core_language_main_test_banner(String::from("stage1")));
        } else {
            println!("{}", String::from("bad"));
        }
        while false {
        }
    });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(payload) if payload.is::<AxiomRuntimeAbort>() => std::process::ExitCode::from(1),
        Err(_) => {
            axiom_runtime_report("panic", "runtime panic");
            std::process::ExitCode::from(1)
        }
    }
}
