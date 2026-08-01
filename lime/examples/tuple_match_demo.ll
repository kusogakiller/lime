; ModuleID = 'lime'
source_filename = "lime"
target triple = "x86_64-pc-windows-msvc"

%Result = type { i32, [1 x i64] }
%Option = type { i32, [1 x i64] }
%LimeList = type { i8*, i64, i64 }
%LimeOption = type { i1, i8* }
%LimeIface = type { i8*, i8* }

declare i8* @runtime_alloc(i64, i64)
declare void @runtime_print(i8*)
declare i64 @strlen(i8*)
declare i8* @runtime_str_slice(i8*, i64, i64)
declare i8* @runtime_str_concat(i8*, i8*)
declare %LimeList @runtime_str_chars(i8*)
declare %LimeList @runtime_str_bytes(i8*)
declare %LimeList @runtime_list_add(%LimeList, i64)
declare %LimeList @runtime_list_set(%LimeList, i64, i64)

@.str.true   = private unnamed_addr constant [5 x i8] c"true\00"
@.str.false  = private unnamed_addr constant [6 x i8] c"false\00"
@.str.newline = private unnamed_addr constant [2 x i8] c"\0A\00"
@.str.lbracket = private unnamed_addr constant [2 x i8] c"[\00"
@.str.rbracket = private unnamed_addr constant [2 x i8] c"]\00"
@.str.space  = private unnamed_addr constant [2 x i8] c" \00"

declare i8* @_i64toa(i64, ptr, i32)
declare i8* @_gcvt(double, i32, ptr)
declare i8* @_itoa(i32, ptr, i32)
@.str.2 = private unnamed_addr constant [9 x i8] c"wildcard\00"
@.str.1 = private unnamed_addr constant [6 x i8] c"seven\00"
@.str.0 = private unnamed_addr constant [3 x i8] c"hi\00"


; Function main()
define i32 @main_lime () nounwind uwtable {
L0:
  %t0 = getelementptr inbounds [3 x i8], ptr @.str.0, i64 0, i64 0
  %t1 = insertvalue {i32, i8*} undef, i32 1, 0
  %t2 = insertvalue {i32, i8*} %t1, i8* %t0, 1
  %t3 = alloca {i32, i8*}, align 8
  store {i32, i8*} %t2, {i32, i8*}* %t3, align 8
  %t4 = load {i32, i8*}, {i32, i8*}* %t3, align 8
  br label %L2
L2:
  %t5 = extractvalue {i32, i8*} %t4, 0
  %t6 = alloca i32, align 4
  store i32 %t5, i32* %t6, align 4
  %t7 = extractvalue {i32, i8*} %t4, 1
  %t8 = alloca i8*, align 8
  store i8* %t7, i8** %t8, align 8
  %t9 = load i32, i32* %t6, align 4
  %t10 = alloca i8, i64 12, align 1
  call i8* @_itoa(i32 %t9, ptr %t10, i32 10)
  call void @runtime_print(ptr %t10)
  call void @runtime_print(ptr @.str.newline)
  %t11 = load i8*, i8** %t8, align 8
  call void @runtime_print(ptr %t11)
  call void @runtime_print(ptr @.str.newline)
  br label %L1
L1:
  %t12 = insertvalue {i32, i32} undef, i32 20, 0
  %t13 = insertvalue {i32, i32} %t12, i32 30, 1
  %t14 = insertvalue {i32, {i32, i32}} undef, i32 10, 0
  %t15 = insertvalue {i32, {i32, i32}} %t14, {i32, i32} %t13, 1
  %t16 = alloca {i32, {i32, i32}}, align 4
  store {i32, {i32, i32}} %t15, {i32, {i32, i32}}* %t16, align 4
  %t17 = load {i32, {i32, i32}}, {i32, {i32, i32}}* %t16, align 4
  br label %L4
L4:
  %t18 = extractvalue {i32, {i32, i32}} %t17, 0
  %t19 = alloca i32, align 4
  store i32 %t18, i32* %t19, align 4
  %t20 = extractvalue {i32, {i32, i32}} %t17, 1
  %t21 = extractvalue {i32, i32} %t20, 0
  %t22 = alloca i32, align 4
  store i32 %t21, i32* %t22, align 4
  %t23 = extractvalue {i32, i32} %t20, 1
  %t24 = alloca i32, align 4
  store i32 %t23, i32* %t24, align 4
  %t25 = load i32, i32* %t19, align 4
  %t26 = load i32, i32* %t22, align 4
  %t27 = add nsw i32 %t25, %t26
  %t28 = load i32, i32* %t24, align 4
  %t29 = add nsw i32 %t27, %t28
  %t30 = alloca i8, i64 12, align 1
  call i8* @_itoa(i32 %t29, ptr %t30, i32 10)
  call void @runtime_print(ptr %t30)
  call void @runtime_print(ptr @.str.newline)
  br label %L3
L3:
  %t31 = getelementptr inbounds [6 x i8], ptr @.str.1, i64 0, i64 0
  %t32 = insertvalue {i32, i8*} undef, i32 7, 0
  %t33 = insertvalue {i32, i8*} %t32, i8* %t31, 1
  %t34 = alloca {i32, i8*}, align 8
  store {i32, i8*} %t33, {i32, i8*}* %t34, align 8
  %t35 = load {i32, i8*}, {i32, i8*}* %t34, align 8
  br label %L6
L6:
  %t36 = getelementptr inbounds [9 x i8], ptr @.str.2, i64 0, i64 0
  call void @runtime_print(ptr %t36)
  call void @runtime_print(ptr @.str.newline)
  br label %L5
L5:
  %t37 = call {i32, i32} @swap.int.int(i32 1, i32 2)
  %t38 = alloca {i32, i32}, align 4
  store {i32, i32} %t37, {i32, i32}* %t38, align 4
  %t39 = load {i32, i32}, {i32, i32}* %t38, align 4
  br label %L8
L8:
  %t40 = extractvalue {i32, i32} %t39, 0
  %t41 = alloca i32, align 4
  store i32 %t40, i32* %t41, align 4
  %t42 = extractvalue {i32, i32} %t39, 1
  %t43 = alloca i32, align 4
  store i32 %t42, i32* %t43, align 4
  %t44 = load i32, i32* %t43, align 4
  %t45 = alloca i8, i64 12, align 1
  call i8* @_itoa(i32 %t44, ptr %t45, i32 10)
  call void @runtime_print(ptr %t45)
  call void @runtime_print(ptr @.str.newline)
  %t46 = load i32, i32* %t41, align 4
  %t47 = alloca i8, i64 12, align 1
  call i8* @_itoa(i32 %t46, ptr %t47, i32 10)
  call void @runtime_print(ptr %t47)
  call void @runtime_print(ptr @.str.newline)
  br label %L7
L7:
  ret i32 0
}

; Function swap.int.int()
define {i32, i32} @swap.int.int (i32 %p0, i32 %p1) nounwind uwtable {
L0:
  %t0 = alloca i32, align 4
  store i32 %p0, i32* %t0, align 4
  %t1 = alloca i32, align 4
  store i32 %p1, i32* %t1, align 4
  %t2 = load i32, i32* %t1, align 4
  %t3 = load i32, i32* %t0, align 4
  %t4 = insertvalue {i32, i32} undef, i32 %t2, 0
  %t5 = insertvalue {i32, i32} %t4, i32 %t3, 1
  ret {i32, i32} %t5
}
define i32 @main() nounwind uwtable {
  call void @main_lime()
  ret i32 0
}

