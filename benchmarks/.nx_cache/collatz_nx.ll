; NextLang Module: collatz_nx
target triple = "x86_64-redhat-linux-gnu"

; String Literals
@__str_0 = private unnamed_addr constant [21 x i8] c"Collatz total steps:\00", align 1

; Runtime Function Declarations
declare void @nx_rt_vout_str(ptr)
declare void @nx_rt_vout_int(i64)
declare void @nx_rt_vout_float(double)
declare void @nx_rt_vout_bool(i1)
declare void @nx_rt_vout_char(i32)
declare void @nx_rt_vout_space()
declare void @nx_rt_vout_newline()
declare ptr @nx_rt_input(ptr)
declare i64 @nx_rt_to_int(ptr)
declare double @nx_rt_to_float(ptr)
declare i64 @nx_rt_len_str(ptr)
declare ptr @nx_rt_alloc(i64)
declare void @nx_rt_free(ptr)
declare void @nx_rt_panic(ptr)
declare ptr @nx_rt_str_concat(ptr, ptr)
declare i1 @nx_rt_str_eq(ptr, ptr)

; Phase 8 Systems Primitives
declare ptr @buffer_new(i64)
declare void @buffer_free(ptr)
declare i64 @buffer_size(ptr)
declare i8 @buffer_get_u8(ptr, i64)
declare void @buffer_set_u8(ptr, i64, i8)
declare i32 @buffer_get_u32_le(ptr, i64)
declare void @buffer_set_u32_le(ptr, i64, i32)
declare i32 @buffer_get_i32_le(ptr, i64)
declare void @buffer_set_i32_le(ptr, i64, i32)
declare i64 @file_open(ptr, ptr)
declare i1 @file_close(i64)
declare ptr @file_read_all(ptr)
declare i1 @file_write(ptr, ptr)
declare i1 @file_exists(ptr)
declare i1 @file_remove(ptr)
declare ptr @env_get(ptr)
declare i1 @env_set(ptr, ptr)
declare ptr @get_cwd()
declare i32 @get_pid()
declare ptr @ptr_offset(ptr, i64)
declare i8 @ptr_read_u8(ptr)
declare void @ptr_write_u8(ptr, i8)

define i32 @main() {
entry:
  %t0 = alloca i64
  store i64 0, ptr %t0
  %t1 = alloca i64
  store i64 1, ptr %t1
  br label %for_header_0
for_header_0:
  %t2 = load i64, ptr %t1
  %t3 = icmp sle i64 %t2, 50000
  br i1 %t3, label %for_body_1, label %for_exit_3
for_body_1:
  %t4 = alloca i64
  %t5 = load i64, ptr %t1
  store i64 %t5, ptr %t4
  br label %as_header_4
as_header_4:
  %t6 = load i64, ptr %t4
  %t7 = icmp sgt i64 %t6, 1
  br i1 %t7, label %as_body_5, label %as_exit_6
as_body_5:
  %t8 = load i64, ptr %t4
  %t9 = srem i64 %t8, 2
  %t10 = icmp eq i64 %t9, 0
  br i1 %t10, label %if_then_7, label %if_otherwise_or_else_9
if_then_7:
  %t11 = load i64, ptr %t4
  %t12 = sdiv i64 %t11, 2
  store i64 %t12, ptr %t4
  br label %if_merge_8
if_otherwise_or_else_9:
  %t13 = load i64, ptr %t4
  %t14 = mul i64 %t13, 3
  %t15 = add i64 %t14, 1
  store i64 %t15, ptr %t4
  br label %if_merge_8
if_merge_8:
  %t16 = load i64, ptr %t0
  %t17 = add i64 %t16, 1
  store i64 %t17, ptr %t0
  br label %as_header_4
as_exit_6:
  br label %for_update_2
for_update_2:
  %t18 = load i64, ptr %t1
  %t19 = add i64 %t18, 1
  store i64 %t19, ptr %t1
  br label %for_header_0
for_exit_3:
  %t20 = load i64, ptr %t0
  call void @nx_rt_vout_str(ptr @__str_0)
  call void @nx_rt_vout_space()
  call void @nx_rt_vout_int(i64 %t20)
  call void @nx_rt_vout_newline()
  ret i32 0
}

