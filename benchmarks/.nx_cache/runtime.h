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

#ifdef __cplusplus
}
#endif

#endif // NX_RUNTIME_H
