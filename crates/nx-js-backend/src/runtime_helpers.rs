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

    helpers
}
