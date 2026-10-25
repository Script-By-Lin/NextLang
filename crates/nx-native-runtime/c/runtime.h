#ifndef NX_RUNTIME_H
#define NX_RUNTIME_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Standard output functions
void nx_rt_vout_str(const char *str);
void nx_rt_vout_int(int64_t val);
void nx_rt_vout_float(double val);
void nx_rt_vout_bool(int8_t val);
void nx_rt_vout_char(int32_t ch);
void nx_rt_vout_space(void);
void nx_rt_vout_newline(void);

// Standard input function
const char* nx_rt_input(const char *prompt);

// Explicit conversions
int64_t nx_rt_to_int(const char *str);
double nx_rt_to_float(const char *str);

// Length functions
int64_t nx_rt_len_str(const char *str);

// Memory management
void* nx_rt_alloc(int64_t size);
void nx_rt_free(void *ptr);

// Runtime error and abort
void nx_rt_panic(const char *msg);

// String helpers
const char* nx_rt_str_concat(const char *a, const char *b);
int8_t nx_rt_str_eq(const char *a, const char *b);

// File I/O
int64_t nx_rt_file_open(const char *path, const char *mode);
int8_t nx_rt_file_close(int64_t handle);
const char* nx_rt_file_read_all(const char *path);
int8_t nx_rt_file_write(const char *path, const char *content);
int8_t nx_rt_file_exists(const char *path);
int8_t nx_rt_file_remove(const char *path);

// OS Environment and Process
const char* nx_rt_env_get(const char *key);
int8_t nx_rt_env_set(const char *key, const char *val);
const char* nx_rt_get_cwd(void);
int32_t nx_rt_get_pid(void);

// Byte Buffer
void* nx_rt_buffer_new(uint64_t size);
void nx_rt_buffer_free(void *buf);
uint64_t nx_rt_buffer_size(void *buf);
uint8_t nx_rt_buffer_get_u8(void *buf, uint64_t offset);
void nx_rt_buffer_set_u8(void *buf, uint64_t offset, uint8_t val);
uint32_t nx_rt_buffer_get_u32_le(void *buf, uint64_t offset);
void nx_rt_buffer_set_u32_le(void *buf, uint64_t offset, uint32_t val);
int32_t nx_rt_buffer_get_i32_le(void *buf, uint64_t offset);
void nx_rt_buffer_set_i32_le(void *buf, uint64_t offset, int32_t val);

// Raw Pointer Operations
void* nx_rt_ptr_offset(void *p, int64_t bytes);
uint8_t nx_rt_ptr_read_u8(void *p);
void nx_rt_ptr_write_u8(void *p, uint8_t val);

// System built-in aliases
void* buffer_new(uint64_t size);
void buffer_free(void *buf);
uint64_t buffer_size(void *buf);
uint8_t buffer_get_u8(void *buf, uint64_t offset);
void buffer_set_u8(void *buf, uint64_t offset, uint8_t val);
uint32_t buffer_get_u32_le(void *buf, uint64_t offset);
void buffer_set_u32_le(void *buf, uint64_t offset, uint32_t val);
int32_t buffer_get_i32_le(void *buf, uint64_t offset);
void buffer_set_i32_le(void *buf, uint64_t offset, int32_t val);

int64_t file_open(const char *path, const char *mode);
int8_t file_close(int64_t handle);
const char* file_read_all(const char *path);
int8_t file_write(const char *path, const char *content);
int8_t file_exists(const char *path);
int8_t file_remove(const char *path);

const char* env_get(const char *key);
int8_t env_set(const char *key, const char *val);
const char* get_cwd(void);
int32_t get_pid(void);

void* ptr_offset(void *p, int64_t bytes);
uint8_t ptr_read_u8(void *p);
void ptr_write_u8(void *p, uint8_t val);

// Phase 9 Standard Library Primitives
// Math
double math_abs(double x);
int64_t int_abs(int64_t x);
double math_sqrt(double x);
double math_pow(double base, double exp);
double math_floor(double x);
double math_ceil(double x);
double math_min(double a, double b);
double math_max(double a, double b);
double math_sin(double x);
double math_cos(double x);
double math_random(void);

// String
int64_t str_len(const char *s);
const char* str_upper(const char *s);
const char* str_lower(const char *s);
const char* str_trim(const char *s);
const char* str_sub(const char *s, int64_t start, int64_t len);
int8_t str_contains(const char *s, const char *sub);
int8_t str_starts_with(const char *s, const char *prefix);
int8_t str_ends_with(const char *s, const char *suffix);

// Time
int64_t time_now_ms(void);
void time_sleep_ms(int64_t ms);

// Encoding
const char* hex_encode(const char *s);
const char* hex_decode(const char *s);
const char* base64_encode(const char *s);
const char* base64_decode(const char *s);

// JSON
const char* json_parse(const char *s);
const char* json_stringify(const char *s);

// Testing & Assertions
void nx_rt_assert(int8_t cond, const char *msg);
void nx_rt_assert_eq(const char *actual, const char *expected, const char *msg);
void assert(int8_t cond, const char *msg);
void assert_eq(const char *actual, const char *expected, const char *msg);

// Phase 10: Asynchronous Runtime & Network I/O
void nx_rt_async_sleep(int64_t ms);
int64_t nx_rt_tcp_listen(const char *host, int32_t port);
int64_t nx_rt_tcp_accept(int64_t server_fd);
int64_t nx_rt_tcp_connect(const char *host, int32_t port);
int64_t nx_rt_tcp_read(int64_t fd, char *buf, int64_t max_len);
const char* nx_rt_tcp_read_str(int64_t fd, int64_t max_len);
int64_t nx_rt_tcp_write(int64_t fd, const char *data, int64_t len);
int64_t nx_rt_tcp_write_str(int64_t fd, const char *data);
void nx_rt_tcp_close(int64_t fd);

void* nx_rt_task_spawn(void* (*fn)(void*), void *arg);
void* nx_rt_task_join(void *task);
int8_t nx_rt_task_cancel(void *task);

// Standard aliases
void async_sleep(int64_t ms);
int64_t tcp_listen(const char *host, int32_t port);
int64_t tcp_accept(int64_t server_fd);
int64_t tcp_connect(const char *host, int32_t port);
int64_t tcp_read(int64_t fd, char *buf, int64_t max_len);
const char* tcp_read_str(int64_t fd, int64_t max_len);
int64_t tcp_write(int64_t fd, const char *data, int64_t len);
int64_t tcp_write_str(int64_t fd, const char *data);
void tcp_close(int64_t fd);
void* task_spawn(void* (*fn)(void*), void *arg);
void* task_join(void *task);
int8_t task_cancel(void *task);

// Phase 11: HTTP & NextAPI Runtime Primitives
const char* nx_rt_http_parse(const char *raw);
const char* http_parse(const char *raw);
const char* nx_rt_url_decode(const char *s);
const char* url_decode(const char *s);
const char* nx_rt_url_encode(const char *s);
const char* url_encode(const char *s);
const char* nx_rt_http_status_message(int32_t code);
const char* http_status_message(int32_t code);
const char* nx_rt_int_to_str(int64_t v);
const char* int_to_str(int64_t v);

#ifdef __cplusplus
}
#endif

#endif // NX_RUNTIME_H
