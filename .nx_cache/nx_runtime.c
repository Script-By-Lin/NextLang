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
