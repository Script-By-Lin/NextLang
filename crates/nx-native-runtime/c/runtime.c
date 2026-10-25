#include "runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <math.h>
#include <errno.h>
#include <time.h>
#include <unistd.h>
#include <fcntl.h>
#include <poll.h>
#include <pthread.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>

void nx_rt_vout_str(const char *str) {
    if (str) {
        fputs(str, stdout);
    }
}

void nx_rt_vout_int(int64_t val) {
    printf("%lld", (long long)val);
}

void nx_rt_vout_float(double val) {
    char buf[64];
    if (__builtin_floor(val) == val && !isinf(val) && !isnan(val)) {
        snprintf(buf, sizeof(buf), "%.1f", val);
    } else {
        snprintf(buf, sizeof(buf), "%g", val);
    }
    printf("%s", buf);
}

void nx_rt_vout_bool(int8_t val) {
    if (val) {
        printf("true");
    } else {
        printf("false");
    }
}

void nx_rt_vout_char(int32_t ch) {
    putchar((int)ch);
}

void nx_rt_vout_space(void) {
    putchar(' ');
}

void nx_rt_vout_newline(void) {
    putchar('\n');
    fflush(stdout);
}

const char* nx_rt_input(const char *prompt) {
    if (prompt && *prompt) {
        fputs(prompt, stdout);
        fflush(stdout);
    }

    char *line = NULL;
    size_t cap = 0;
    ssize_t nread = getline(&line, &cap, stdin);

    if (nread < 0) {
        if (line) free(line);
        char *empty = (char*)malloc(1);
        empty[0] = '\0';
        return empty;
    }

    // Strip trailing \r and \n
    while (nread > 0 && (line[nread - 1] == '\n' || line[nread - 1] == '\r')) {
        line[--nread] = '\0';
    }

    return line;
}

int64_t nx_rt_to_int(const char *str) {
    if (!str || !*str) {
        nx_rt_panic("invalid integer format: empty string");
    }

    const char *p = str;
    while (*p && isspace((unsigned char)*p)) p++;

    errno = 0;
    char *endptr = NULL;
    long long val = strtoll(p, &endptr, 10);

    if (errno != 0 || endptr == p) {
        nx_rt_panic("invalid integer format");
    }

    while (*endptr && isspace((unsigned char)*endptr)) endptr++;
    if (*endptr != '\0') {
        nx_rt_panic("invalid integer format");
    }

    return (int64_t)val;
}

double nx_rt_to_float(const char *str) {
    if (!str || !*str) {
        nx_rt_panic("invalid float format: empty string");
    }

    const char *p = str;
    while (*p && isspace((unsigned char)*p)) p++;

    errno = 0;
    char *endptr = NULL;
    double val = strtod(p, &endptr);

    if (errno != 0 || endptr == p) {
        nx_rt_panic("invalid float format");
    }

    while (*endptr && isspace((unsigned char)*endptr)) endptr++;
    if (*endptr != '\0') {
        nx_rt_panic("invalid float format");
    }

    return val;
}

int64_t nx_rt_len_str(const char *str) {
    return str ? (int64_t)strlen(str) : 0;
}

void* nx_rt_alloc(int64_t size) {
    if (size <= 0) size = 8;
    void *ptr = malloc((size_t)size);
    if (!ptr) {
        nx_rt_panic("out of memory in alloc()");
    }
    memset(ptr, 0, (size_t)size);
    return ptr;
}

void nx_rt_free(void *ptr) {
    if (ptr) {
        free(ptr);
    }
}

void nx_rt_panic(const char *msg) {
    fprintf(stderr, "Runtime Error: %s\n", msg ? msg : "unknown");
    fflush(stderr);
    exit(1);
}

const char* nx_rt_str_concat(const char *a, const char *b) {
    size_t la = a ? strlen(a) : 0;
    size_t lb = b ? strlen(b) : 0;
    char *buf = (char*)malloc(la + lb + 1);
    if (!buf) nx_rt_panic("out of memory in string concatenation");
    if (la > 0) memcpy(buf, a, la);
    if (lb > 0) memcpy(buf + la, b, lb);
    buf[la + lb] = '\0';
    return buf;
}

int8_t nx_rt_str_eq(const char *a, const char *b) {
    if (a == b) return 1;
    if (!a || !b) return 0;
    return strcmp(a, b) == 0 ? 1 : 0;
}

#include <unistd.h>

typedef struct {
    uint64_t size;
    uint8_t *data;
} nx_buffer_t;

void* nx_rt_buffer_new(uint64_t size) {
    nx_buffer_t *b = (nx_buffer_t*)malloc(sizeof(nx_buffer_t));
    if (!b) nx_rt_panic("out of memory in buffer_new");
    b->size = size;
    b->data = (uint8_t*)malloc(size > 0 ? size : 1);
    if (!b->data) nx_rt_panic("out of memory in buffer_new data");
    memset(b->data, 0, size);
    return b;
}

void nx_rt_buffer_free(void *buf) {
    if (buf) {
        nx_buffer_t *b = (nx_buffer_t*)buf;
        if (b->data) free(b->data);
        free(b);
    }
}

uint64_t nx_rt_buffer_size(void *buf) {
    if (!buf) return 0;
    return ((nx_buffer_t*)buf)->size;
}

uint8_t nx_rt_buffer_get_u8(void *buf, uint64_t offset) {
    if (!buf) nx_rt_panic("null pointer dereference in buffer_get_u8");
    nx_buffer_t *b = (nx_buffer_t*)buf;
    if (offset >= b->size) nx_rt_panic("buffer index out of bounds in buffer_get_u8");
    return b->data[offset];
}

void nx_rt_buffer_set_u8(void *buf, uint64_t offset, uint8_t val) {
    if (!buf) nx_rt_panic("null pointer dereference in buffer_set_u8");
    nx_buffer_t *b = (nx_buffer_t*)buf;
    if (offset >= b->size) nx_rt_panic("buffer index out of bounds in buffer_set_u8");
    b->data[offset] = val;
}

uint32_t nx_rt_buffer_get_u32_le(void *buf, uint64_t offset) {
    if (!buf) nx_rt_panic("null pointer dereference in buffer_get_u32_le");
    nx_buffer_t *b = (nx_buffer_t*)buf;
    if (offset + 4 > b->size) nx_rt_panic("buffer index out of bounds in buffer_get_u32_le");
    uint8_t *p = b->data + offset;
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

void nx_rt_buffer_set_u32_le(void *buf, uint64_t offset, uint32_t val) {
    if (!buf) nx_rt_panic("null pointer dereference in buffer_set_u32_le");
    nx_buffer_t *b = (nx_buffer_t*)buf;
    if (offset + 4 > b->size) nx_rt_panic("buffer index out of bounds in buffer_set_u32_le");
    uint8_t *p = b->data + offset;
    p[0] = (uint8_t)(val & 0xFF);
    p[1] = (uint8_t)((val >> 8) & 0xFF);
    p[2] = (uint8_t)((val >> 16) & 0xFF);
    p[3] = (uint8_t)((val >> 24) & 0xFF);
}

int32_t nx_rt_buffer_get_i32_le(void *buf, uint64_t offset) {
    return (int32_t)nx_rt_buffer_get_u32_le(buf, offset);
}

void nx_rt_buffer_set_i32_le(void *buf, uint64_t offset, int32_t val) {
    nx_rt_buffer_set_u32_le(buf, offset, (uint32_t)val);
}

int64_t nx_rt_file_open(const char *path, const char *mode) {
    if (!path || !mode) return 0;
    FILE *f = fopen(path, mode);
    return (int64_t)(uintptr_t)f;
}

int8_t nx_rt_file_close(int64_t handle) {
    if (handle == 0) return 0;
    FILE *f = (FILE*)(uintptr_t)handle;
    return fclose(f) == 0 ? 1 : 0;
}

const char* nx_rt_file_read_all(const char *path) {
    if (!path) return "";
    FILE *f = fopen(path, "rb");
    if (!f) return "";
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    if (sz < 0) {
        fclose(f);
        return "";
    }
    char *buf = (char*)malloc(sz + 1);
    if (!buf) {
        fclose(f);
        nx_rt_panic("out of memory in file_read_all");
    }
    size_t n = fread(buf, 1, sz, f);
    buf[n] = '\0';
    fclose(f);
    return buf;
}

int8_t nx_rt_file_write(const char *path, const char *content) {
    if (!path || !content) return 0;
    FILE *f = fopen(path, "wb");
    if (!f) return 0;
    size_t len = strlen(content);
    size_t written = fwrite(content, 1, len, f);
    fclose(f);
    return written == len ? 1 : 0;
}

int8_t nx_rt_file_exists(const char *path) {
    if (!path) return 0;
    return access(path, F_OK) == 0 ? 1 : 0;
}

int8_t nx_rt_file_remove(const char *path) {
    if (!path) return 0;
    return remove(path) == 0 ? 1 : 0;
}

const char* nx_rt_env_get(const char *key) {
    if (!key) return "";
    const char *val = getenv(key);
    if (!val) return "";
    char *dup = strdup(val);
    return dup ? dup : "";
}

int8_t nx_rt_env_set(const char *key, const char *val) {
    if (!key || !val) return 0;
    return setenv(key, val, 1) == 0 ? 1 : 0;
}

const char* nx_rt_get_cwd(void) {
    char buf[1024];
    if (getcwd(buf, sizeof(buf))) {
        char *dup = strdup(buf);
        return dup ? dup : "";
    }
    return "";
}

int32_t nx_rt_get_pid(void) {
    return (int32_t)getpid();
}

void* nx_rt_ptr_offset(void *p, int64_t bytes) {
    return (void*)((char*)p + bytes);
}

uint8_t nx_rt_ptr_read_u8(void *p) {
    if (!p) nx_rt_panic("null pointer dereference in ptr_read_u8");
    return *(uint8_t*)p;
}

void nx_rt_ptr_write_u8(void *p, uint8_t val) {
    if (!p) nx_rt_panic("null pointer dereference in ptr_write_u8");
    *(uint8_t*)p = val;
}

// System built-in aliases
void* buffer_new(uint64_t size) { return nx_rt_buffer_new(size); }
void buffer_free(void *buf) { nx_rt_buffer_free(buf); }
uint64_t buffer_size(void *buf) { return nx_rt_buffer_size(buf); }
uint8_t buffer_get_u8(void *buf, uint64_t offset) { return nx_rt_buffer_get_u8(buf, offset); }
void buffer_set_u8(void *buf, uint64_t offset, uint8_t val) { nx_rt_buffer_set_u8(buf, offset, val); }
uint32_t buffer_get_u32_le(void *buf, uint64_t offset) { return nx_rt_buffer_get_u32_le(buf, offset); }
void buffer_set_u32_le(void *buf, uint64_t offset, uint32_t val) { nx_rt_buffer_set_u32_le(buf, offset, val); }
int32_t buffer_get_i32_le(void *buf, uint64_t offset) { return nx_rt_buffer_get_i32_le(buf, offset); }
void buffer_set_i32_le(void *buf, uint64_t offset, int32_t val) { nx_rt_buffer_set_i32_le(buf, offset, val); }

int64_t file_open(const char *path, const char *mode) { return nx_rt_file_open(path, mode); }
int8_t file_close(int64_t handle) { return nx_rt_file_close(handle); }
const char* file_read_all(const char *path) { return nx_rt_file_read_all(path); }
int8_t file_write(const char *path, const char *content) { return nx_rt_file_write(path, content); }
int8_t file_exists(const char *path) { return nx_rt_file_exists(path); }
int8_t file_remove(const char *path) { return nx_rt_file_remove(path); }

const char* env_get(const char *key) { return nx_rt_env_get(key); }
int8_t env_set(const char *key, const char *val) { return nx_rt_env_set(key, val); }
const char* get_cwd(void) { return nx_rt_get_cwd(); }
int32_t get_pid(void) { return nx_rt_get_pid(); }

void* ptr_offset(void *p, int64_t bytes) { return nx_rt_ptr_offset(p, bytes); }
uint8_t ptr_read_u8(void *p) { return nx_rt_ptr_read_u8(p); }
void ptr_write_u8(void *p, uint8_t val) { nx_rt_ptr_write_u8(p, val); }

// Phase 9 Standard Library Primitives
// Math
double math_abs(double x) {
    return fabs(x);
}

int64_t int_abs(int64_t x) {
    return llabs((long long)x);
}

double math_sqrt(double x) {
    if (x < 0.0) {
        nx_rt_panic("math domain error: sqrt of negative number");
    }
    return __builtin_sqrt(x);
}

double math_pow(double base, double exp) {
    return __builtin_pow(base, exp);
}

double math_floor(double x) {
    return __builtin_floor(x);
}

double math_ceil(double x) {
    return __builtin_ceil(x);
}

double math_min(double a, double b) {
    return (a < b) ? a : b;
}

double math_max(double a, double b) {
    return (a > b) ? a : b;
}

double math_sin(double x) {
    return __builtin_sin(x);
}

double math_cos(double x) {
    return __builtin_cos(x);
}

static int g_rand_seeded = 0;
double math_random(void) {
    if (!g_rand_seeded) {
        srand((unsigned int)time(NULL));
        g_rand_seeded = 1;
    }
    return (double)rand() / ((double)RAND_MAX + 1.0);
}

// String
int64_t str_len(const char *s) {
    return s ? (int64_t)strlen(s) : 0;
}

const char* str_upper(const char *s) {
    if (!s) return "";
    size_t len = strlen(s);
    char *res = (char*)malloc(len + 1);
    if (!res) nx_rt_panic("out of memory in str_upper");
    for (size_t i = 0; i < len; i++) {
        res[i] = (char)toupper((unsigned char)s[i]);
    }
    res[len] = '\0';
    return res;
}

const char* str_lower(const char *s) {
    if (!s) return "";
    size_t len = strlen(s);
    char *res = (char*)malloc(len + 1);
    if (!res) nx_rt_panic("out of memory in str_lower");
    for (size_t i = 0; i < len; i++) {
        res[i] = (char)tolower((unsigned char)s[i]);
    }
    res[len] = '\0';
    return res;
}

const char* str_trim(const char *s) {
    if (!s) return "";
    while (*s && isspace((unsigned char)*s)) s++;
    if (*s == '\0') {
        char *empty = (char*)malloc(1);
        empty[0] = '\0';
        return empty;
    }
    const char *end = s + strlen(s) - 1;
    while (end > s && isspace((unsigned char)*end)) end--;
    size_t len = (size_t)(end - s + 1);
    char *res = (char*)malloc(len + 1);
    if (!res) nx_rt_panic("out of memory in str_trim");
    memcpy(res, s, len);
    res[len] = '\0';
    return res;
}

const char* str_sub(const char *s, int64_t start, int64_t len) {
    if (!s || len <= 0) return "";
    int64_t slen = (int64_t)strlen(s);
    if (start < 0) start = 0;
    if (start >= slen) return "";
    if (start + len > slen) len = slen - start;
    char *res = (char*)malloc((size_t)len + 1);
    if (!res) nx_rt_panic("out of memory in str_sub");
    memcpy(res, s + start, (size_t)len);
    res[len] = '\0';
    return res;
}

int8_t str_contains(const char *s, const char *sub) {
    if (!s || !sub) return 0;
    return strstr(s, sub) != NULL ? 1 : 0;
}

int8_t str_starts_with(const char *s, const char *prefix) {
    if (!s || !prefix) return 0;
    size_t plen = strlen(prefix);
    return strncmp(s, prefix, plen) == 0 ? 1 : 0;
}

int8_t str_ends_with(const char *s, const char *suffix) {
    if (!s || !suffix) return 0;
    size_t slen = strlen(s);
    size_t xlen = strlen(suffix);
    if (xlen > slen) return 0;
    return strcmp(s + slen - xlen, suffix) == 0 ? 1 : 0;
}

// Time
int64_t time_now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return ((int64_t)ts.tv_sec * 1000) + ((int64_t)ts.tv_nsec / 1000000);
}

void time_sleep_ms(int64_t ms) {
    if (ms > 0) {
        usleep((useconds_t)(ms * 1000));
    }
}

// Encoding
const char* hex_encode(const char *s) {
    if (!s) return "";
    static const char hex_chars[] = "0123456789abcdef";
    size_t len = strlen(s);
    char *res = (char*)malloc(len * 2 + 1);
    if (!res) nx_rt_panic("out of memory in hex_encode");
    for (size_t i = 0; i < len; i++) {
        unsigned char b = (unsigned char)s[i];
        res[i * 2] = hex_chars[b >> 4];
        res[i * 2 + 1] = hex_chars[b & 0x0F];
    }
    res[len * 2] = '\0';
    return res;
}

const char* hex_decode(const char *s) {
    if (!s) return "";
    size_t len = strlen(s);
    if (len % 2 != 0) {
        nx_rt_panic("invalid hex string: odd length");
    }
    size_t out_len = len / 2;
    char *res = (char*)malloc(out_len + 1);
    if (!res) nx_rt_panic("out of memory in hex_decode");
    for (size_t i = 0; i < out_len; i++) {
        char high = s[i * 2];
        char low = s[i * 2 + 1];
        int h = isdigit((unsigned char)high) ? high - '0' : (tolower((unsigned char)high) - 'a' + 10);
        int l = isdigit((unsigned char)low) ? low - '0' : (tolower((unsigned char)low) - 'a' + 10);
        if (h < 0 || h > 15 || l < 0 || l > 15) {
            nx_rt_panic("invalid hex character in hex_decode");
        }
        res[i] = (char)((h << 4) | l);
    }
    res[out_len] = '\0';
    return res;
}

static const char b64_table[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

const char* base64_encode(const char *s) {
    if (!s) return "";
    size_t len = strlen(s);
    size_t out_len = 4 * ((len + 2) / 3);
    char *res = (char*)malloc(out_len + 1);
    if (!res) nx_rt_panic("out of memory in base64_encode");
    size_t i = 0, j = 0;
    while (i < len) {
        uint32_t oct_a = i < len ? (unsigned char)s[i++] : 0;
        uint32_t oct_b = i < len ? (unsigned char)s[i++] : 0;
        uint32_t oct_c = i < len ? (unsigned char)s[i++] : 0;
        uint32_t triple = (oct_a << 16) | (oct_b << 8) | oct_c;
        res[j++] = b64_table[(triple >> 18) & 0x3F];
        res[j++] = b64_table[(triple >> 12) & 0x3F];
        res[j++] = (i > len + 1) ? '=' : b64_table[(triple >> 6) & 0x3F];
        res[j++] = (i > len) ? '=' : b64_table[triple & 0x3F];
    }
    res[out_len] = '\0';
    return res;
}

static int b64_val(char c) {
    if (c >= 'A' && c <= 'Z') return c - 'A';
    if (c >= 'a' && c <= 'z') return c - 'a' + 26;
    if (c >= '0' && c <= '9') return c - '0' + 52;
    if (c == '+') return 62;
    if (c == '/') return 63;
    return -1;
}

const char* base64_decode(const char *s) {
    if (!s) return "";
    size_t len = strlen(s);
    if (len % 4 != 0) {
        nx_rt_panic("invalid base64 string length");
    }
    size_t pad = 0;
    if (len > 0 && s[len - 1] == '=') pad++;
    if (len > 1 && s[len - 2] == '=') pad++;
    size_t out_len = (len / 4) * 3 - pad;
    char *res = (char*)malloc(out_len + 1);
    if (!res) nx_rt_panic("out of memory in base64_decode");
    size_t i = 0, j = 0;
    while (i < len) {
        int v0 = b64_val(s[i++]);
        int v1 = b64_val(s[i++]);
        int v2 = s[i] == '=' ? 0 : b64_val(s[i]); i++;
        int v3 = s[i] == '=' ? 0 : b64_val(s[i]); i++;
        if (v0 < 0 || v1 < 0) {
            nx_rt_panic("invalid base64 character");
        }
        uint32_t triple = ((uint32_t)v0 << 18) | ((uint32_t)v1 << 12) | ((uint32_t)v2 << 6) | (uint32_t)v3;
        if (j < out_len) res[j++] = (char)((triple >> 16) & 0xFF);
        if (j < out_len) res[j++] = (char)((triple >> 8) & 0xFF);
        if (j < out_len) res[j++] = (char)(triple & 0xFF);
    }
    res[out_len] = '\0';
    return res;
}

// JSON
static void skip_json_whitespace(const char **p) {
    while (**p && isspace((unsigned char)**p)) (*p)++;
}

static int validate_json_value(const char **p);

static int validate_json_string(const char **p) {
    if (**p != '"') return 0;
    (*p)++;
    while (**p && **p != '"') {
        if (**p == '\\') {
            (*p)++;
            if (!**p) return 0;
        }
        (*p)++;
    }
    if (**p == '"') {
        (*p)++;
        return 1;
    }
    return 0;
}

static int validate_json_number(const char **p) {
    if (**p == '-') (*p)++;
    if (!isdigit((unsigned char)**p)) return 0;
    while (isdigit((unsigned char)**p)) (*p)++;
    if (**p == '.') {
        (*p)++;
        if (!isdigit((unsigned char)**p)) return 0;
        while (isdigit((unsigned char)**p)) (*p)++;
    }
    if (**p == 'e' || **p == 'E') {
        (*p)++;
        if (**p == '+' || **p == '-') (*p)++;
        if (!isdigit((unsigned char)**p)) return 0;
        while (isdigit((unsigned char)**p)) (*p)++;
    }
    return 1;
}

static int validate_json_array(const char **p) {
    if (**p != '[') return 0;
    (*p)++;
    skip_json_whitespace(p);
    if (**p == ']') {
        (*p)++;
        return 1;
    }
    while (**p) {
        if (!validate_json_value(p)) return 0;
        skip_json_whitespace(p);
        if (**p == ']') {
            (*p)++;
            return 1;
        }
        if (**p != ',') return 0;
        (*p)++;
        skip_json_whitespace(p);
    }
    return 0;
}

static int validate_json_object(const char **p) {
    if (**p != '{') return 0;
    (*p)++;
    skip_json_whitespace(p);
    if (**p == '}') {
        (*p)++;
        return 1;
    }
    while (**p) {
        skip_json_whitespace(p);
        if (!validate_json_string(p)) return 0;
        skip_json_whitespace(p);
        if (**p != ':') return 0;
        (*p)++;
        skip_json_whitespace(p);
        if (!validate_json_value(p)) return 0;
        skip_json_whitespace(p);
        if (**p == '}') {
            (*p)++;
            return 1;
        }
        if (**p != ',') return 0;
        (*p)++;
        skip_json_whitespace(p);
    }
    return 0;
}

static int validate_json_value(const char **p) {
    skip_json_whitespace(p);
    if (**p == '{') return validate_json_object(p);
    if (**p == '[') return validate_json_array(p);
    if (**p == '"') return validate_json_string(p);
    if (strncmp(*p, "true", 4) == 0) { *p += 4; return 1; }
    if (strncmp(*p, "false", 5) == 0) { *p += 5; return 1; }
    if (strncmp(*p, "null", 4) == 0) { *p += 4; return 1; }
    return validate_json_number(p);
}

const char* json_parse(const char *s) {
    if (!s) nx_rt_panic("json_parse error: null input");
    const char *p = s;
    skip_json_whitespace(&p);
    if (!validate_json_value(&p)) {
        nx_rt_panic("json_parse error: malformed JSON");
    }
    skip_json_whitespace(&p);
    if (*p != '\0') {
        nx_rt_panic("json_parse error: trailing characters after JSON value");
    }
    return strdup(s);
}

const char* json_stringify(const char *s) {
    if (!s) return "null";
    return strdup(s);
}

// Testing & Assertions
void nx_rt_assert(int8_t cond, const char *msg) {
    if (!cond) {
        fprintf(stderr, "Assertion Failed: %s\n", msg ? msg : "assertion check failed");
        fflush(stderr);
        exit(1);
    }
}

void nx_rt_assert_eq(const char *actual, const char *expected, const char *msg) {
    if (!actual || !expected || strcmp(actual, expected) != 0) {
        fprintf(stderr, "Assertion Failed: %s\n  expected: '%s'\n  actual:   '%s'\n",
                msg ? msg : "values are not equal",
                expected ? expected : "null",
                actual ? actual : "null");
        fflush(stderr);
        exit(1);
    }
}

void assert(int8_t cond, const char *msg) {
    nx_rt_assert(cond, msg);
}

void assert_eq(const char *actual, const char *expected, const char *msg) {
    nx_rt_assert_eq(actual, expected, msg);
}

// Phase 10: Asynchronous Runtime & Network I/O Implementation

typedef struct NxTask {
    pthread_t thread;
    void* (*fn)(void*);
    void *arg;
    void *result;
    volatile int8_t completed;
    volatile int8_t cancelled;
} NxTask;

static void* nx_rt_task_worker(void *data) {
    NxTask *task = (NxTask *)data;
    if (task && !task->cancelled && task->fn) {
        task->result = task->fn(task->arg);
    }
    if (task) {
        task->completed = 1;
    }
    return NULL;
}

void nx_rt_async_sleep(int64_t ms) {
    if (ms <= 0) return;
    poll(NULL, 0, (int)ms);
}

int64_t nx_rt_tcp_listen(const char *host, int32_t port) {
    char host_buf[256] = {0};
    int actual_port = port;
    if (host) {
        const char *colon = strrchr(host, ':');
        if (colon) {
            size_t hlen = (size_t)(colon - host);
            if (hlen < sizeof(host_buf)) {
                strncpy(host_buf, host, hlen);
                host_buf[hlen] = '\0';
            }
            actual_port = atoi(colon + 1);
        } else {
            int p = atoi(host);
            if (p > 0 && port == 0) {
                actual_port = p;
                strcpy(host_buf, "0.0.0.0");
            } else {
                strncpy(host_buf, host, sizeof(host_buf) - 1);
            }
        }
    }

    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return -1;
    int opt = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
    int flags = fcntl(fd, F_GETFL, 0);
    fcntl(fd, F_SETFL, flags | O_NONBLOCK);

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)actual_port);
    if (host_buf[0] == '\0' || strcmp(host_buf, "0.0.0.0") == 0 || strcmp(host_buf, "localhost") == 0) {
        addr.sin_addr.s_addr = INADDR_ANY;
    } else {
        inet_pton(AF_INET, host_buf, &addr.sin_addr);
    }

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        close(fd);
        return -1;
    }
    if (listen(fd, 128) < 0) {
        close(fd);
        return -1;
    }
    return (int64_t)fd;
}

int64_t nx_rt_tcp_accept(int64_t server_fd) {
    if (server_fd < 0) return -1;
    struct pollfd pfd;
    pfd.fd = (int)server_fd;
    pfd.events = POLLIN;
    int ret = poll(&pfd, 1, 5000);
    if (ret <= 0) return -1;

    struct sockaddr_in client_addr;
    socklen_t client_len = sizeof(client_addr);
    int client_fd = accept((int)server_fd, (struct sockaddr *)&client_addr, &client_len);
    if (client_fd < 0) return -1;

    int flags = fcntl(client_fd, F_GETFL, 0);
    fcntl(client_fd, F_SETFL, flags | O_NONBLOCK);
    return (int64_t)client_fd;
}

int64_t nx_rt_tcp_connect(const char *host, int32_t port) {
    char host_buf[256] = {0};
    int actual_port = port;
    if (host) {
        const char *colon = strrchr(host, ':');
        if (colon) {
            size_t hlen = (size_t)(colon - host);
            if (hlen < sizeof(host_buf)) {
                strncpy(host_buf, host, hlen);
                host_buf[hlen] = '\0';
            }
            actual_port = atoi(colon + 1);
        } else {
            strncpy(host_buf, host, sizeof(host_buf) - 1);
        }
    }

    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return -1;

    int flags = fcntl(fd, F_GETFL, 0);
    fcntl(fd, F_SETFL, flags | O_NONBLOCK);

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)actual_port);
    const char *target_ip = (host_buf[0] && strcmp(host_buf, "localhost") != 0 && strcmp(host_buf, "0.0.0.0") != 0) ? host_buf : "127.0.0.1";
    inet_pton(AF_INET, target_ip, &addr.sin_addr);

    int res = connect(fd, (struct sockaddr *)&addr, sizeof(addr));
    if (res < 0) {
        if (errno == EINPROGRESS) {
            struct pollfd pfd;
            pfd.fd = fd;
            pfd.events = POLLOUT;
            int pret = poll(&pfd, 1, 5000);
            if (pret <= 0) {
                close(fd);
                return -1;
            }
            int err = 0;
            socklen_t len = sizeof(err);
            if (getsockopt(fd, SOL_SOCKET, SO_ERROR, &err, &len) < 0 || err != 0) {
                close(fd);
                return -1;
            }
        } else {
            close(fd);
            return -1;
        }
    }
    return (int64_t)fd;
}

int64_t nx_rt_tcp_read(int64_t fd, char *buf, int64_t max_len) {
    if (fd < 0 || !buf || max_len <= 0) return -1;
    struct pollfd pfd;
    pfd.fd = (int)fd;
    pfd.events = POLLIN;
    int pret = poll(&pfd, 1, 5000);
    if (pret <= 0) return 0;
    ssize_t n = recv((int)fd, buf, (size_t)max_len, 0);
    return (int64_t)n;
}

const char* nx_rt_tcp_read_str(int64_t fd, int64_t max_len) {
    if (fd < 0 || max_len <= 0) return "";
    char *buf = (char *)malloc((size_t)max_len + 1);
    if (!buf) return "";
    int64_t n = nx_rt_tcp_read(fd, buf, max_len);
    if (n <= 0) {
        free(buf);
        return "";
    }
    buf[n] = '\0';
    return buf;
}

int64_t nx_rt_tcp_write(int64_t fd, const char *data, int64_t len) {
    if (fd < 0 || !data || len <= 0) return -1;
    struct pollfd pfd;
    pfd.fd = (int)fd;
    pfd.events = POLLOUT;
    int pret = poll(&pfd, 1, 5000);
    if (pret <= 0) return 0;
    ssize_t n = send((int)fd, data, (size_t)len, 0);
    return (int64_t)n;
}

int64_t nx_rt_tcp_write_str(int64_t fd, const char *data) {
    if (!data) return 0;
    return nx_rt_tcp_write(fd, data, (int64_t)strlen(data));
}

void nx_rt_tcp_close(int64_t fd) {
    if (fd >= 0) {
        close((int)fd);
    }
}

void* nx_rt_task_spawn(void* (*fn)(void*), void *arg) {
    NxTask *task = (NxTask *)malloc(sizeof(NxTask));
    if (!task) return NULL;
    task->fn = fn;
    task->arg = arg;
    task->result = NULL;
    task->completed = 0;
    task->cancelled = 0;
    if (pthread_create(&task->thread, NULL, nx_rt_task_worker, task) != 0) {
        free(task);
        return NULL;
    }
    return task;
}

void* nx_rt_task_join(void *task_ptr) {
    NxTask *task = (NxTask *)task_ptr;
    if (!task) return NULL;
    pthread_join(task->thread, NULL);
    void *res = task->result;
    free(task);
    return res;
}

int8_t nx_rt_task_cancel(void *task_ptr) {
    NxTask *task = (NxTask *)task_ptr;
    if (!task) return 0;
    task->cancelled = 1;
    return 1;
}

void async_sleep(int64_t ms) { nx_rt_async_sleep(ms); }
int64_t tcp_listen(const char *host, int32_t port) { return nx_rt_tcp_listen(host, port); }
int64_t tcp_accept(int64_t server_fd) { return nx_rt_tcp_accept(server_fd); }
int64_t tcp_connect(const char *host, int32_t port) { return nx_rt_tcp_connect(host, port); }
int64_t tcp_read(int64_t fd, char *buf, int64_t max_len) { return nx_rt_tcp_read(fd, buf, max_len); }
const char* tcp_read_str(int64_t fd, int64_t max_len) { return nx_rt_tcp_read_str(fd, max_len); }
int64_t tcp_write(int64_t fd, const char *data, int64_t len) { return nx_rt_tcp_write(fd, data, len); }
int64_t tcp_write_str(int64_t fd, const char *data) { return nx_rt_tcp_write_str(fd, data); }
void tcp_close(int64_t fd) { nx_rt_tcp_close(fd); }
void* task_spawn(void* (*fn)(void*), void *arg) { return nx_rt_task_spawn(fn, arg); }
void* task_join(void *task) { return nx_rt_task_join(task); }
int8_t task_cancel(void *task) { return nx_rt_task_cancel(task); }

// Phase 11: HTTP & NextAPI Runtime Primitives
const char* nx_rt_url_decode(const char *s) {
    if (!s) return "";
    size_t len = strlen(s);
    char *out = (char*)malloc(len + 1);
    if (!out) nx_rt_panic("out of memory in url_decode");
    size_t o = 0;
    for (size_t i = 0; i < len; i++) {
        if (s[i] == '+') {
            out[o++] = ' ';
        } else if (s[i] == '%' && i + 2 < len && isxdigit((unsigned char)s[i+1]) && isxdigit((unsigned char)s[i+2])) {
            char hex[3] = { s[i+1], s[i+2], '\0' };
            out[o++] = (char)strtol(hex, NULL, 16);
            i += 2;
        } else {
            out[o++] = s[i];
        }
    }
    out[o] = '\0';
    return out;
}

const char* nx_rt_url_encode(const char *s) {
    if (!s) return "";
    size_t len = strlen(s);
    char *out = (char*)malloc(len * 3 + 1);
    if (!out) nx_rt_panic("out of memory in url_encode");
    static const char hex_chars[] = "0123456789ABCDEF";
    size_t o = 0;
    for (size_t i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        if (isalnum(c) || c == '-' || c == '_' || c == '.' || c == '~') {
            out[o++] = (char)c;
        } else {
            out[o++] = '%';
            out[o++] = hex_chars[c >> 4];
            out[o++] = hex_chars[c & 0x0F];
        }
    }
    out[o] = '\0';
    return out;
}

const char* nx_rt_http_status_message(int32_t code) {
    switch (code) {
        case 200: return "OK";
        case 201: return "Created";
        case 202: return "Accepted";
        case 204: return "No Content";
        case 301: return "Moved Permanently";
        case 302: return "Found";
        case 304: return "Not Modified";
        case 307: return "Temporary Redirect";
        case 308: return "Permanent Redirect";
        case 400: return "Bad Request";
        case 401: return "Unauthorized";
        case 403: return "Forbidden";
        case 404: return "Not Found";
        case 405: return "Method Not Allowed";
        case 409: return "Conflict";
        case 413: return "Payload Too Large";
        case 415: return "Unsupported Media Type";
        case 422: return "Unprocessable Entity";
        case 500: return "Internal Server Error";
        case 501: return "Not Implemented";
        case 502: return "Bad Gateway";
        case 503: return "Service Unavailable";
        default: return "Unknown";
    }
}

const char* nx_rt_http_parse(const char *raw) {
    if (!raw || strlen(raw) == 0) {
        return "{\"valid\":false,\"method\":\"\",\"path\":\"\",\"query\":\"\",\"headers\":{},\"body\":\"\",\"content_length\":0,\"error\":\"empty request\"}";
    }

    const char *line_end = strstr(raw, "\r\n");
    size_t line_end_len = 2;
    if (!line_end) {
        line_end = strchr(raw, '\n');
        line_end_len = 1;
    }
    if (!line_end) {
        return "{\"valid\":false,\"method\":\"\",\"path\":\"\",\"query\":\"\",\"headers\":{},\"body\":\"\",\"content_length\":0,\"error\":\"malformed request: no newline\"}";
    }

    char line_buf[2048] = {0};
    size_t line_len = (size_t)(line_end - raw);
    if (line_len >= sizeof(line_buf)) {
        return "{\"valid\":false,\"method\":\"\",\"path\":\"\",\"query\":\"\",\"headers\":{},\"body\":\"\",\"content_length\":0,\"error\":\"request line too long\"}";
    }
    strncpy(line_buf, raw, line_len);
    line_buf[line_len] = '\0';

    char method_buf[32] = {0};
    char uri_buf[2048] = {0};
    char ver_buf[32] = {0};
    if (sscanf(line_buf, "%31s %2047s %31s", method_buf, uri_buf, ver_buf) < 2) {
        return "{\"valid\":false,\"method\":\"\",\"path\":\"\",\"query\":\"\",\"headers\":{},\"body\":\"\",\"content_length\":0,\"error\":\"malformed request line\"}";
    }

    if (strcmp(method_buf, "GET") != 0 &&
        strcmp(method_buf, "POST") != 0 &&
        strcmp(method_buf, "PUT") != 0 &&
        strcmp(method_buf, "PATCH") != 0 &&
        strcmp(method_buf, "DELETE") != 0 &&
        strcmp(method_buf, "HEAD") != 0 &&
        strcmp(method_buf, "OPTIONS") != 0) {
        return "{\"valid\":false,\"method\":\"\",\"path\":\"\",\"query\":\"\",\"headers\":{},\"body\":\"\",\"content_length\":0,\"error\":\"unsupported HTTP method\"}";
    }

    char path_buf[2048] = {0};
    char query_buf[2048] = {0};
    char *qmark = strchr(uri_buf, '?');
    if (qmark) {
        size_t plen = (size_t)(qmark - uri_buf);
        if (plen >= sizeof(path_buf)) plen = sizeof(path_buf) - 1;
        memcpy(path_buf, uri_buf, plen);
        path_buf[plen] = '\0';
        snprintf(query_buf, sizeof(query_buf), "%s", qmark + 1);
    } else {
        snprintf(path_buf, sizeof(path_buf), "%s", uri_buf);
    }

    const char *header_start = line_end + line_end_len;
    const char *body_start = strstr(header_start, "\r\n\r\n");
    size_t boundary_len = 4;
    if (!body_start) {
        body_start = strstr(header_start, "\n\n");
        boundary_len = 2;
    }
    if (!body_start) {
        body_start = header_start + strlen(header_start);
        boundary_len = 0;
    }

    size_t json_cap = 16384 + strlen(raw);
    char *json = (char*)malloc(json_cap);
    if (!json) nx_rt_panic("out of memory in http_parse");

    char *h_str = (char*)malloc(header_start < body_start ? (size_t)(body_start - header_start + 1) : 1);
    if (!h_str) nx_rt_panic("out of memory in http_parse");
    if (header_start < body_start) {
        memcpy(h_str, header_start, (size_t)(body_start - header_start));
        h_str[body_start - header_start] = '\0';
    } else {
        h_str[0] = '\0';
    }

    char headers_json[8192] = "{";
    size_t hj_len = 1;
    int first_header = 1;
    int64_t content_length = 0;
    int has_transfer_encoding = 0;
    int content_length_count = 0;

    char *saveptr = NULL;
    char *h_line = strtok_r(h_str, "\r\n", &saveptr);
    while (h_line) {
        char *colon = strchr(h_line, ':');
        if (colon) {
            *colon = '\0';
            char *h_key = h_line;
            char *h_val = colon + 1;
            while (*h_key && isspace((unsigned char)*h_key)) h_key++;
            char *kend = h_key + strlen(h_key);
            while (kend > h_key && isspace((unsigned char)*(kend - 1))) kend--;
            *kend = '\0';

            while (*h_val && isspace((unsigned char)*h_val)) h_val++;
            char *vend = h_val + strlen(h_val);
            while (vend > h_val && isspace((unsigned char)*(vend - 1))) vend--;
            *vend = '\0';

            for (char *c = h_key; *c; c++) *c = (char)tolower((unsigned char)*c);

            if (strcmp(h_key, "content-length") == 0) {
                content_length_count++;
                content_length = atoll(h_val);
                if (content_length_count > 1) {
                    free(h_str);
                    free(json);
                    return "{\"valid\":false,\"method\":\"\",\"path\":\"\",\"query\":\"\",\"headers\":{},\"body\":\"\",\"content_length\":0,\"error\":\"request smuggling risk: multiple Content-Length headers\"}";
                }
            } else if (strcmp(h_key, "transfer-encoding") == 0) {
                has_transfer_encoding = 1;
            }

            if (!first_header) {
                if (hj_len + 1 < sizeof(headers_json)) headers_json[hj_len++] = ',';
            }
            first_header = 0;

            int written = snprintf(headers_json + hj_len, sizeof(headers_json) - hj_len, "\"%s\":\"", h_key);
            if (written > 0) hj_len += (size_t)written;
            for (char *v = h_val; *v && hj_len + 4 < sizeof(headers_json); v++) {
                if (*v == '"' || *v == '\\') {
                    headers_json[hj_len++] = '\\';
                }
                headers_json[hj_len++] = *v;
            }
            if (hj_len + 1 < sizeof(headers_json)) headers_json[hj_len++] = '"';
        }
        h_line = strtok_r(NULL, "\r\n", &saveptr);
    }
    free(h_str);

    if (hj_len + 1 < sizeof(headers_json)) headers_json[hj_len++] = '}';
    headers_json[hj_len] = '\0';

    if (content_length_count > 0 && has_transfer_encoding) {
        free(json);
        return "{\"valid\":false,\"method\":\"\",\"path\":\"\",\"query\":\"\",\"headers\":{},\"body\":\"\",\"content_length\":0,\"error\":\"request smuggling risk: both Content-Length and Transfer-Encoding present\"}";
    }

    const char *raw_body = boundary_len > 0 ? (body_start + boundary_len) : "";
    size_t actual_body_len = strlen(raw_body);
    if (content_length > 0 && (size_t)content_length < actual_body_len) {
        actual_body_len = (size_t)content_length;
    }

    char *escaped_body = (char*)malloc(actual_body_len * 2 + 1);
    if (!escaped_body) nx_rt_panic("out of memory in http_parse");
    size_t eb_len = 0;
    for (size_t i = 0; i < actual_body_len; i++) {
        char c = raw_body[i];
        if (c == '"' || c == '\\') {
            escaped_body[eb_len++] = '\\';
            escaped_body[eb_len++] = c;
        } else if (c == '\n') {
            escaped_body[eb_len++] = '\\';
            escaped_body[eb_len++] = 'n';
        } else if (c == '\r') {
            escaped_body[eb_len++] = '\\';
            escaped_body[eb_len++] = 'r';
        } else if (c == '\t') {
            escaped_body[eb_len++] = '\\';
            escaped_body[eb_len++] = 't';
        } else if ((unsigned char)c < 32) {
            eb_len += (size_t)snprintf(escaped_body + eb_len, 7, "\\u%04x", (unsigned char)c);
        } else {
            escaped_body[eb_len++] = c;
        }
    }
    escaped_body[eb_len] = '\0';

    snprintf(json, json_cap,
        "{\"valid\":true,\"method\":\"%s\",\"path\":\"%s\",\"query\":\"%s\",\"headers\":%s,\"body\":\"%s\",\"content_length\":%lld,\"error\":\"\"}",
        method_buf, path_buf, query_buf, headers_json, escaped_body, (long long)actual_body_len);

    free(escaped_body);
    return json;
}

const char* http_parse(const char *raw) { return nx_rt_http_parse(raw); }
const char* url_decode(const char *s) { return nx_rt_url_decode(s); }
const char* url_encode(const char *s) { return nx_rt_url_encode(s); }
const char* http_status_message(int32_t code) { return nx_rt_http_status_message(code); }

const char* nx_rt_int_to_str(int64_t v) {
    char buf[64];
    snprintf(buf, sizeof(buf), "%lld", (long long)v);
    return strdup(buf);
}
const char* int_to_str(int64_t v) { return nx_rt_int_to_str(v); }


