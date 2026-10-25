use crate::config::JsTarget;

/// Returns the embedded JavaScript runtime helper functions based on target.
pub fn get_runtime_helpers(target: JsTarget) -> String {
    let mut helpers = String::new();

    // len helper
    helpers.push_str(
        r#"function __nx_len(x) {
    if (x === null || x === undefined) return 0;
    if (typeof x === 'string' || Array.isArray(x)) return x.length;
    if (x instanceof Map || x instanceof Set) return x.size;
    if (typeof x === 'object') return Object.keys(x).length;
    return 0;
}
"#,
    );

    // to_int helper
    helpers.push_str(
        r#"function __nx_to_int(x) {
    if (typeof x === 'number') return Math.trunc(x);
    const n = parseInt(String(x).trim(), 10);
    if (Number.isNaN(n)) throw new Error("invalid integer format: '" + x + "'");
    return n;
}
"#,
    );

    // to_float helper
    helpers.push_str(
        r#"function __nx_to_float(x) {
    if (typeof x === 'number') return x;
    const n = parseFloat(String(x).trim());
    if (Number.isNaN(n)) throw new Error("invalid float format: '" + x + "'");
    return n;
}
"#,
    );

    // alloc and free helpers
    helpers.push_str(
        r#"function __nx_alloc(x) {
    return { slot: 0, val: x };
}
function __nx_free(x) {
    if (x && typeof x === 'object') {
        x.val = null;
    }
}
"#,
    );

    // Target-specific input helper
    match target {
        JsTarget::Node => {
            helpers.push_str(
                r#"function __nx_input(prompt) {
    if (prompt !== undefined && prompt !== null) {
        process.stdout.write(String(prompt));
    }
    const fs = require('fs');
    const buf = Buffer.alloc(1);
    let line = '';
    while (true) {
        let bytesRead = 0;
        try {
            bytesRead = fs.readSync(0, buf, 0, 1, null);
        } catch (e) {
            break;
        }
        if (bytesRead === 0) break;
        const char = buf.toString('utf8', 0, 1);
        if (char === '\n') break;
        if (char !== '\r') line += char;
    }
    return line;
}
"#,
            );
        }
        JsTarget::Browser => {
            helpers.push_str(
                r#"function __nx_input(prompt) {
    if (typeof window !== 'undefined' && typeof window.prompt === 'function') {
        return window.prompt(prompt ? String(prompt) : '') || '';
    }
    throw new Error("interactive synchronous input is not supported in this browser context");
}
"#,
            );
        }
    }

    // Phase 9 Standard Library runtime helpers
    helpers.push_str(
        r#"function math_abs(x) { return Math.abs(x); }
function int_abs(x) { return Math.abs(Math.trunc(x)); }
function math_sqrt(x) {
    if (x < 0) throw new Error("math domain error: sqrt of negative number");
    return Math.sqrt(x);
}
function math_pow(b, e) { return Math.pow(b, e); }
function math_floor(x) { return Math.floor(x); }
function math_ceil(x) { return Math.ceil(x); }
function math_min(a, b) { return Math.min(a, b); }
function math_max(a, b) { return Math.max(a, b); }
function math_sin(x) { return Math.sin(x); }
function math_cos(x) { return Math.cos(x); }
function math_random() { return Math.random(); }

function str_len(s) { return String(s).length; }
function str_upper(s) { return String(s).toUpperCase(); }
function str_lower(s) { return String(s).toLowerCase(); }
function str_trim(s) { return String(s).trim(); }
function str_sub(s, start, len) {
    const str = String(s);
    return str.substring(start, start + len);
}
function str_contains(s, sub) { return String(s).includes(String(sub)); }
function str_starts_with(s, prefix) { return String(s).startsWith(String(prefix)); }
function str_ends_with(s, suffix) { return String(s).endsWith(String(suffix)); }

function time_now_ms() { return Date.now(); }
function time_sleep_ms(ms) {
    const start = Date.now();
    while (Date.now() - start < ms) {}
}

function hex_encode(s) {
    const str = String(s);
    let hex = '';
    for (let i = 0; i < str.length; i++) {
        hex += str.charCodeAt(i).toString(16).padStart(2, '0');
    }
    return hex;
}
function hex_decode(s) {
    const str = String(s);
    if (str.length % 2 !== 0) throw new Error("invalid hex string");
    let res = '';
    for (let i = 0; i < str.length; i += 2) {
        res += String.fromCharCode(parseInt(str.substring(i, i + 2), 16));
    }
    return res;
}
function base64_encode(s) {
    if (typeof Buffer !== 'undefined') return Buffer.from(String(s), 'utf8').toString('base64');
    return btoa(String(s));
}
function base64_decode(s) {
    if (typeof Buffer !== 'undefined') return Buffer.from(String(s), 'base64').toString('utf8');
    return atob(String(s));
}

function json_parse(s) {
    JSON.parse(String(s));
    return String(s);
}
function json_stringify(s) {
    return JSON.stringify(s);
}

function assert(cond, msg) {
    if (!cond) throw new Error("Assertion Failed: " + (msg || "assertion failed"));
}
function assert_eq(a, b, msg) {
    if (a !== b) throw new Error("Assertion Failed" + (msg ? ": " + msg : "") + ": expected '" + b + "', got '" + a + "'");
}

// Phase 10 Asynchronous Runtime Helpers
function async_sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, Math.max(0, ms || 0)));
}
function sleep(ms) {
    return async_sleep(ms);
}
function task_spawn(fut) {
    return Promise.resolve(fut);
}
function task_join(task) {
    return Promise.resolve(task);
}
function task_cancel(task) {
    return false;
}
function timeout(ms, fut) {
    return Promise.race([
        Promise.resolve(fut),
        new Promise((_, reject) => setTimeout(() => reject(new Error("operation timed out")), ms))
    ]);
}
let __nx_next_sock_id = 1;
function tcp_listen(addr) {
    return Promise.resolve(__nx_next_sock_id++);
}
function tcp_accept(id) {
    return Promise.resolve(__nx_next_sock_id++);
}
function tcp_connect(addr) {
    return Promise.resolve(__nx_next_sock_id++);
}
function tcp_read(id, max_len) {
    return Promise.resolve("");
}
function tcp_write(id, data) {
    return Promise.resolve(String(data).length);
}
function tcp_close(id) {
    return undefined;
}
function url_decode(s) {
    try { return decodeURIComponent(String(s).replace(/\+/g, ' ')); } catch (_) { return String(s); }
}
function url_encode(s) {
    return encodeURIComponent(String(s));
}
function http_status_message(code) {
    const map = { 200: "OK", 201: "Created", 204: "No Content", 400: "Bad Request", 401: "Unauthorized", 403: "Forbidden", 404: "Not Found", 405: "Method Not Allowed", 413: "Payload Too Large", 422: "Unprocessable Entity", 500: "Internal Server Error" };
    return map[code] || "Unknown";
}
function http_parse(raw) {
    const s = String(raw || "");
    const boundary = s.indexOf("\r\n\r\n") !== -1 ? s.indexOf("\r\n\r\n") : s.indexOf("\n\n");
    const hPart = boundary !== -1 ? s.slice(0, boundary) : s;
    const bPart = boundary !== -1 ? s.slice(boundary + (s.indexOf("\r\n\r\n") !== -1 ? 4 : 2)) : "";
    const lines = hPart.split(/\r?\n/);
    const first = (lines[0] || "").split(/\s+/);
    if (first.length < 2) return JSON.stringify({ valid: false, method: "", path: "", query: "", headers: {}, body: "", content_length: 0, error: "malformed request" });
    const method = first[0];
    const uriParts = (first[1] || "").split("?");
    const path = uriParts[0];
    const query = uriParts[1] || "";
    const headers = {};
    for (let i = 1; i < lines.length; i++) {
        const c = lines[i].indexOf(":");
        if (c !== -1) {
            headers[lines[i].slice(0, c).trim().toLowerCase()] = lines[i].slice(c + 1).trim();
        }
    }
    return JSON.stringify({ valid: true, method, path, query, headers, body: bPart, content_length: bPart.length, error: "" });
}
function int_to_str(v) {
    return String(v);
}
"#,
    );

    helpers
}
