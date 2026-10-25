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
"#,
    );

    helpers
}
