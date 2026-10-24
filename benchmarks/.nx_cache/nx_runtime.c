#include "runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <math.h>
#include <errno.h>

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
    if (floor(val) == val && !isinf(val) && !isnan(val)) {
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

