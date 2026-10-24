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

#ifdef __cplusplus
}
#endif

#endif // NX_RUNTIME_H
