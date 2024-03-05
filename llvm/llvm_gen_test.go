// Code generated by testgen. DO NOT EDIT.
package llvm

import (
	"fmt"
	"testing"
	"strings"

	"github.com/stretchr/testify/require"
)
			
func Test10PromptCodegen(t *testing.T) {
	input := `
@printf("What is your name?\n")
@prompt(10, (name: @str){
    @printf("Hello, %s!\n", name)
})

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [20 x i8] c"What is your name?\0A\00"
@builtin.prompt$10.format = private unnamed_addr constant [5 x i8] c"%10s\00"
@1 = private unnamed_addr constant [12 x i8] c"Hello, %s!\0A\00"

define i32 @main() {
entry:
	call void @printf$(i8* getelementptr ([20 x i8], [20 x i8]* @0, i32 0, i32 0))
	call void @builtin.prompt$10(i32 10, void (i8*)* @main.0)
	ret i32 0
}

define void @printf$(i8* %fmt) {
entry:
	%0 = call i32 (i8*, ...) @printf(i8* %fmt)
	ret void
}

declare i32 @printf(i8* %fmt, ...)

declare i32 @__isoc99_scanf(i8* %fmt, ...)

define void @builtin.prompt$10(void (i8*)* %limit, void (i8*)* %ok) {
entry:
	%0 = alloca [11 x i8]
	%1 = getelementptr [11 x i8], [11 x i8]* %0, i32 0, i32 0
	%2 = call i32 (i8*, ...) @__isoc99_scanf(i8* getelementptr ([5 x i8], [5 x i8]* @builtin.prompt$10.format, i32 0, i32 0), i8* %1)
	call void %ok(i8* %1)
	ret void
}

define void @main.0(i8* %name) {
entry:
	call void @"printf$JXM="(i8* getelementptr ([12 x i8], [12 x i8]* @1, i32 0, i32 0), i8* %name)
	ret void
}

define void @"printf$JXM="(i8* %fmt, i8* %p0) {
entry:
	%0 = call i32 (i8*, ...) @printf(i8* %fmt, i8* %p0)
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test11Prompt2Codegen(t *testing.T) {
	input := `
@printf("What is your name?\n")
callback1: (title: @str, name: @str){
    @printf("What is your age?\n")
    @prompt(3, (age: @str){
        @printf("Hello %s %s age %s!\n", title, name, age)
    })
}
@prompt(10, callback1("Dear"))

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [20 x i8] c"What is your name?\0A\00"
@1 = private unnamed_addr constant [19 x i8] c"What is your age?\0A\00"
@builtin.prompt$3.format = private unnamed_addr constant [4 x i8] c"%3s\00"
@2 = global ptr null
@3 = global ptr null
@4 = private unnamed_addr constant [21 x i8] c"Hello %s %s age %s!\0A\00"
@5 = private unnamed_addr constant [5 x i8] c"Dear\00"

define i32 @main() {
entry:
	call void @printf$(i8* getelementptr ([20 x i8], [20 x i8]* @0, i32 0, i32 0))
	call void @builtin.prompt$10(i32 10, void (i8*)* @main.0)
	ret i32 0
}

define void @main.callback1(i8* %title, i8* %name) {
entry:
	call void @printf$(i8* getelementptr ([19 x i8], [19 x i8]* @1, i32 0, i32 0))
	store i8* %title, ptr @2
	store i8* %name, ptr @3
	call void @builtin.prompt$3(i32 3, void (i8*)* @main.callback1.0)
	ret void
}

define void @builtin.prompt$3(void (i8*)* %limit, void (i8*)* %ok) {
entry:
	%0 = alloca [4 x i8]
	%1 = getelementptr [4 x i8], [4 x i8]* %0, i32 0, i32 0
	%2 = call i32 (i8*, ...) @__isoc99_scanf(i8* getelementptr ([4 x i8], [4 x i8]* @builtin.prompt$3.format, i32 0, i32 0), i8* %1)
	call void %ok(i8* %1)
	ret void
}

define void @main.callback1.0(i8* %age) {
entry:
	%title = load i8*, ptr @2
	%name = load i8*, ptr @3
	call void @printf$JXMlcyVz(i8* getelementptr ([21 x i8], [21 x i8]* @4, i32 0, i32 0), i8* %title, i8* %name, i8* %age)
	ret void
}

define void @printf$JXMlcyVz(i8* %fmt, i8* %p0, i8* %p1, i8* %p2) {
entry:
	%0 = call i32 (i8*, ...) @printf(i8* %fmt, i8* %p0, i8* %p1, i8* %p2)
	ret void
}

define void @main.0(i8* %name) {
entry:
	call void @main.callback1(i8* getelementptr ([5 x i8], [5 x i8]* @5, i32 0, i32 0), i8* %name)
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test11Prompt3Codegen(t *testing.T) {
	input := `
@printf("What is your name?\n")
@prompt(50, (name: @str){
    @printf("What is your age?\n")
    @prompt(3, (age: @str){
        @printf("Hello, %s!\n", name)
    })
})
`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [20 x i8] c"What is your name?\0A\00"
@builtin.prompt$50.format = private unnamed_addr constant [5 x i8] c"%50s\00"
@1 = private unnamed_addr constant [19 x i8] c"What is your age?\0A\00"
@2 = global ptr null
@3 = private unnamed_addr constant [12 x i8] c"Hello, %s!\0A\00"

define i32 @main() {
entry:
	call void @printf$(i8* getelementptr ([20 x i8], [20 x i8]* @0, i32 0, i32 0))
	call void @builtin.prompt$50(i32 50, void (i8*)* @main.0)
	ret i32 0
}

define void @builtin.prompt$50(void (i8*)* %limit, void (i8*)* %ok) {
entry:
	%0 = alloca [51 x i8]
	%1 = getelementptr [51 x i8], [51 x i8]* %0, i32 0, i32 0
	%2 = call i32 (i8*, ...) @__isoc99_scanf(i8* getelementptr ([5 x i8], [5 x i8]* @builtin.prompt$50.format, i32 0, i32 0), i8* %1)
	call void %ok(i8* %1)
	ret void
}

define void @main.0(i8* %name) {
entry:
	call void @printf$(i8* getelementptr ([19 x i8], [19 x i8]* @1, i32 0, i32 0))
	store i8* %name, ptr @2
	call void @builtin.prompt$3(i32 3, void (i8*)* @main.0.0)
	ret void
}

define void @main.0.0(i8* %age) {
entry:
	%name = load i8*, ptr @2
	call void @"printf$JXM="(i8* getelementptr ([12 x i8], [12 x i8]* @3, i32 0, i32 0), i8* %name)
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test12MathAddCodegen(t *testing.T) {
	input := `
@add(3, 3, (x: @int){
    @printf("x: %d\n", x)   
})

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [7 x i8] c"x: %d\0A\00"

define i32 @main() {
entry:
	call void @builtin.add(i32 3, i32 3, void (i32)* @main.0)
	ret i32 0
}

define void @builtin.add(i32 %x, i32 %y, void (i32)* %ok) {
entry:
	%0 = add i32 %x, %y
	call void %ok(i32 %0)
	ret void
}

define void @main.0(i32 %x) {
entry:
	call void @"printf$JWQ="(i8* getelementptr ([7 x i8], [7 x i8]* @0, i32 0, i32 0), i32 %x)
	ret void
}

define void @"printf$JWQ="(i8* %fmt, i32 %p0) {
entry:
	%0 = call i32 (i8*, ...) @printf(i8* %fmt, i32 %p0)
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test12MathMulCodegen(t *testing.T) {
	input := `
mymul: @mul(3, 3)
mymul((x: @int){
    @printf("x: %d\n", x)   
})

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [7 x i8] c"x: %d\0A\00"

define i32 @main() {
entry:
	call void @main.mymul(void (i32)* @main.0)
	ret i32 0
}

define void @builtin.mul(i32 %x, i32 %y, void (i32)* %ok) {
entry:
	%0 = mul i32 %x, %y
	call void %ok(i32 %0)
	ret void
}

define void @main.mymul(void (i32)* %ok) {
entry:
	call void @builtin.mul(i32 3, i32 3, void (i32)* %ok)
	ret void
}

define void @main.0(i32 %x) {
entry:
	call void @"printf$JWQ="(i8* getelementptr ([7 x i8], [7 x i8]* @0, i32 0, i32 0), i32 %x)
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test13FuncIdCodegen(t *testing.T) {
	input := `
puts: (s: @str){
    @unsafe.libc.puts(s, (code:@int){})
}
puts("hello world!")

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = global ptr null
@1 = private unnamed_addr constant [13 x i8] c"hello world!\00"

define i32 @main() {
entry:
	call void @main.puts(i8* getelementptr ([13 x i8], [13 x i8]* @1, i32 0, i32 0))
	ret i32 0
}

define void @main.puts(i8* %s) {
entry:
	store i8* %s, ptr @0
	call void @unsafe.libc.puts(i8* %s, void (i32)* @main.puts.0)
	ret void
}

define void @unsafe.libc.puts(i8* %str, void (i32)* %ok) {
entry:
	%0 = call i32 @puts(i8* %str)
	call void %ok(i32 %0)
	ret void
}

declare i32 @puts(i8* %str)

define void @main.puts.0(i32 %code) {
entry:
	%s = load i8*, ptr @0
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test14AnonCurryCodegen(t *testing.T) {
	input := `
foo: (msg: @str){
    @unsafe.libc.puts(msg, (code:@int){})
}("hi")
foo()

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = global ptr null
@1 = private unnamed_addr constant [3 x i8] c"hi\00"

define i32 @main() {
entry:
	call void @main.foo()
	ret i32 0
}

define void @main.0(i8* %msg) {
entry:
	store i8* %msg, ptr @0
	call void @unsafe.libc.puts(i8* %msg, void (i32)* @main.0.0)
	ret void
}

define void @main.0.0(i32 %code) {
entry:
	%msg = load i8*, ptr @0
	ret void
}

define void @main.foo() {
entry:
	call void @main.0(i8* getelementptr ([3 x i8], [3 x i8]* @1, i32 0, i32 0))
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test15AnonCallCodegen(t *testing.T) {
	input := `
(msg: @str){
    @unsafe.libc.puts(msg, (code:@int){})
}("hi")

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = global ptr null
@1 = private unnamed_addr constant [3 x i8] c"hi\00"

define i32 @main() {
entry:
	call void @main.0(i8* getelementptr ([3 x i8], [3 x i8]* @1, i32 0, i32 0))
	ret i32 0
}

define void @main.0(i8* %msg) {
entry:
	store i8* %msg, ptr @0
	call void @unsafe.libc.puts(i8* %msg, void (i32)* @main.0.0)
	ret void
}

define void @main.0.0(i32 %code) {
entry:
	%msg = load i8*, ptr @0
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test16ComptimeParamCodegen(t *testing.T) {
	input := `
foo: (s: @str!) {
    @unsafe.libc.puts(s, (code:@int){})
}
foo("hi")

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = global ptr null
@1 = private unnamed_addr constant [3 x i8] c"hi\00"

define i32 @main() {
entry:
	call void @main.foo(i8* getelementptr ([3 x i8], [3 x i8]* @1, i32 0, i32 0))
	ret i32 0
}

define void @main.foo(i8* %s) {
entry:
	store i8* %s, ptr @0
	call void @unsafe.libc.puts(i8* %s, void (i32)* @main.foo.0)
	ret void
}

define void @main.foo.0(i32 %code) {
entry:
	%s = load i8*, ptr @0
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test17AtoiCodegen(t *testing.T) {
	input := `
@unsafe.libc.atoi("123", (x: @int){
    @printf("result: %d\n", x)
})

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [4 x i8] c"123\00"
@1 = private unnamed_addr constant [12 x i8] c"result: %d\0A\00"

define i32 @main() {
entry:
	call void @unsafe.libc.atoi(i8* getelementptr ([4 x i8], [4 x i8]* @0, i32 0, i32 0), void (i32)* @main.0)
	ret i32 0
}

define void @unsafe.libc.atoi(i8* %in, void (i32)* %ok) {
entry:
	%0 = call i32 @atoi(i8* %in)
	call void %ok(i32 %0)
	ret void
}

declare i32 @atoi(i8* %in)

define void @main.0(i32 %x) {
entry:
	call void @"printf$JWQ="(i8* getelementptr ([12 x i8], [12 x i8]* @1, i32 0, i32 0), i32 %x)
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test18GtCodegen(t *testing.T) {
	input := `
@igt(2, 3, (){
    @printf("More\n")
}, (){
    @printf("Less\n")
})
`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [6 x i8] c"More\0A\00"
@1 = private unnamed_addr constant [6 x i8] c"Less\0A\00"

define i32 @main() {
entry:
	call void @builtin.igt(i32 2, i32 3, void ()* @main.0, void ()* @main.1)
	ret i32 0
}

define void @builtin.igt(i32 %x, i32 %y, void ()* %true, void ()* %false) {
entry:
	%0 = icmp sgt i32 %x, %y
	br i1 %0, label %iftrue, label %iffalse

iftrue:
	call void %true()
	ret void

iffalse:
	call void %false()
	ret void
}

define void @main.0() {
entry:
	call void @printf$(i8* getelementptr ([6 x i8], [6 x i8]* @0, i32 0, i32 0))
	ret void
}

define void @main.1() {
entry:
	call void @printf$(i8* getelementptr ([6 x i8], [6 x i8]* @1, i32 0, i32 0))
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test19IfCodegen(t *testing.T) {
	input := `
if: (cond: ((),()), ok:()){
    cond(ok, {})
}
if(@igt(4, 3), {
    @printf("More\n")
})

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = global ptr null
@1 = global ptr null
@2 = private unnamed_addr constant [6 x i8] c"More\0A\00"

define i32 @main() {
entry:
	call void @main.if(void (void ()*, void ()*)* @main.0, void ()* @main.1)
	ret i32 0
}

define void @main.if(void (void ()*, void ()*)* %cond, void ()* %ok) {
entry:
	store void ()* %ok, ptr @0
	store void (void ()*, void ()*)* %cond, ptr @1
	call void %cond(void ()* %ok, void ()* @main.if.0)
	ret void
}

define void @main.if.0() {
entry:
	%ok = load void ()*, ptr @0
	%cond = load void (void ()*, void ()*)*, ptr @1
	ret void
}

define void @main.0(void ()* %true, void ()* %false) {
entry:
	call void @builtin.igt(i32 4, i32 3, void ()* %true, void ()* %false)
	ret void
}

define void @main.1() {
entry:
	call void @printf$(i8* getelementptr ([6 x i8], [6 x i8]* @2, i32 0, i32 0))
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test1PrintCodegen(t *testing.T) {
	input := `
@unsafe.libc.puts("hello world!", (code:@int){})

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [13 x i8] c"hello world!\00"

define i32 @main() {
entry:
	call void @unsafe.libc.puts(i8* getelementptr ([13 x i8], [13 x i8]* @0, i32 0, i32 0), void (i32)* @main.0)
	ret i32 0
}

define void @main.0(i32 %code) {
entry:
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test20AnonImmediateCallCodegen(t *testing.T) {
	input := `
(){
    @printf("Hi\n")
}
`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [4 x i8] c"Hi\0A\00"

define i32 @main() {
entry:
	call void @main.0()
	ret i32 0
}

define void @main.0() {
entry:
	call void @printf$(i8* getelementptr ([4 x i8], [4 x i8]* @0, i32 0, i32 0))
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test21AnonImmediateCallCodegen(t *testing.T) {
	input := `
{
    @printf("Hi\n")
}
`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [4 x i8] c"Hi\0A\00"

define i32 @main() {
entry:
	call void @main.0()
	ret i32 0
}

define void @main.0() {
entry:
	call void @printf$(i8* getelementptr ([4 x i8], [4 x i8]* @0, i32 0, i32 0))
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test2IntConstCodegen(t *testing.T) {
	input := `
x: 12
@printf("%d", x)

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [3 x i8] c"%d\00"

define i32 @main() {
entry:
	call void @"printf$JWQ="(i8* getelementptr ([3 x i8], [3 x i8]* @0, i32 0, i32 0), i32 12)
	ret i32 0
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test2StrConstCodegen(t *testing.T) {
	input := `
msg: "hello world"
@unsafe.libc.puts(msg, (code:@int){})

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@msg = private unnamed_addr constant [12 x i8] c"hello world\00"

define i32 @main() {
entry:
	call void @unsafe.libc.puts([12 x i8]* @msg, void (i32)* @main.0)
	ret i32 0
}

define void @main.0(i32 %code) {
entry:
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test3FuncCodegen(t *testing.T) {
	input := `
msg: "hello world"
foo: (){
    @unsafe.libc.puts(msg, (code:@int){})
}
foo()

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@msg = private unnamed_addr constant [12 x i8] c"hello world\00"

define i32 @main() {
entry:
	call void @main.foo()
	ret i32 0
}

define void @main.foo() {
entry:
	call void @unsafe.libc.puts([12 x i8]* @msg, void (i32)* @main.foo.0)
	ret void
}

define void @main.foo.0(i32 %code) {
entry:
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test3Func2Codegen(t *testing.T) {
	input := `
msg: "hello world"
foo: (s:@str){
    @unsafe.libc.puts(s, (code:@int){})
}
foo(msg)
`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@msg = private unnamed_addr constant [12 x i8] c"hello world\00"
@0 = global ptr null

define i32 @main() {
entry:
	call void @main.foo([12 x i8]* @msg)
	ret i32 0
}

define void @main.foo(i8* %s) {
entry:
	store i8* %s, ptr @0
	call void @unsafe.libc.puts(i8* %s, void (i32)* @main.foo.0)
	ret void
}

define void @main.foo.0(i32 %code) {
entry:
	%s = load i8*, ptr @0
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test4FuncCodegen(t *testing.T) {
	input := `
msg: "hello world"
foo: {
    @unsafe.libc.puts(msg, (code:@int){})
}
foo

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@msg = private unnamed_addr constant [12 x i8] c"hello world\00"

define i32 @main() {
entry:
	call void @main.foo()
	ret i32 0
}

define void @main.foo() {
entry:
	call void @unsafe.libc.puts([12 x i8]* @msg, void (i32)* @main.foo.0)
	ret void
}

define void @main.foo.0(i32 %code) {
entry:
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test5FuncStdCodegen(t *testing.T) {
	input := `
@std
foo: (x:string){
	ok: (s: string){
		@unsafe.libc.puts(s, (code:@int){})
	}
	ok(x)
	ok(x)
}
foo("hello world")

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = global ptr null
@1 = global ptr null
@2 = private unnamed_addr constant [12 x i8] c"hello world\00"

define i32 @main() {
entry:
	call void @std()
	call void @main.foo(i8* getelementptr ([12 x i8], [12 x i8]* @2, i32 0, i32 0))
	ret i32 0
}

define void @std() {
entry:
	ret void
}

define void @main.foo(i8* %x) {
entry:
	store i8* %x, ptr @0
	call void @main.foo.ok(i8* %x)
	call void @main.foo.ok(i8* %x)
	ret void
}

define void @main.foo.ok(i8* %s) {
entry:
	%x = load i8*, ptr @0
	store i8* %s, ptr @1
	call void @unsafe.libc.puts(i8* %s, void (i32)* @main.foo.ok.0)
	ret void
}

define void @main.foo.ok.0(i32 %code) {
entry:
	%x = load i8*, ptr @0
	%s = load i8*, ptr @1
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test6CallbackCodegen(t *testing.T) {
	input := `
msg: "hello world"
foo: (input: @str){
    @unsafe.libc.puts(input, (code:@int){})
}
bar: (cb:(@str)) {
    cb(msg)
}
bar(foo)

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@msg = private unnamed_addr constant [12 x i8] c"hello world\00"
@0 = global ptr null

define i32 @main() {
entry:
	call void @main.bar(void (i8*)* @main.foo)
	ret i32 0
}

define void @main.foo(i8* %input) {
entry:
	store i8* %input, ptr @0
	call void @unsafe.libc.puts(i8* %input, void (i32)* @main.foo.0)
	ret void
}

define void @main.foo.0(i32 %code) {
entry:
	%input = load i8*, ptr @0
	ret void
}

define void @main.bar(void (i8*)* %cb) {
entry:
	call void %cb([12 x i8]* @msg)
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test7ApplyCodegen(t *testing.T) {
	input := `
msg: "hello world"
foo: (input: @str, n: @int){
    @printf("%s x %ld", input, n)
}
bar: (cb:(@int)) {
    cb(42)
}
qux: foo(msg)
bar(qux)

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@msg = private unnamed_addr constant [12 x i8] c"hello world\00"
@0 = private unnamed_addr constant [9 x i8] c"%s x %ld\00"

define i32 @main() {
entry:
	call void @main.bar(void (i32)* @main.qux)
	ret i32 0
}

define void @main.foo(i8* %input, i32 %n) {
entry:
	call void @"printf$JXMlbGQ="(i8* getelementptr ([9 x i8], [9 x i8]* @0, i32 0, i32 0), i8* %input, i32 %n)
	ret void
}

define void @"printf$JXMlbGQ="(i8* %fmt, i8* %p0, i32 %p1) {
entry:
	%0 = call i32 (i8*, ...) @printf(i8* %fmt, i8* %p0, i32 %p1)
	ret void
}

define void @main.bar(void (i32)* %cb) {
entry:
	call void %cb(i32 42)
	ret void
}

define void @main.qux(i32 %n) {
entry:
	call void @main.foo([12 x i8]* @msg, i32 %n)
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test8AppliedCallbackCodegen(t *testing.T) {
	input := `
bar: (m1:@str, m2:@str) {
    @printf("msg1: %s, msg2: %s\n", m1, m2)
}
foo: (cb:(@str)){
    cb("bye")
}
foo(bar("hi"))

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [20 x i8] c"msg1: %s, msg2: %s\0A\00"
@1 = private unnamed_addr constant [4 x i8] c"bye\00"
@2 = private unnamed_addr constant [3 x i8] c"hi\00"

define i32 @main() {
entry:
	call void @main.foo(void (i8*)* @main.0)
	ret i32 0
}

define void @main.bar(i8* %m1, i8* %m2) {
entry:
	call void @"printf$JXMlcw=="(i8* getelementptr ([20 x i8], [20 x i8]* @0, i32 0, i32 0), i8* %m1, i8* %m2)
	ret void
}

define void @"printf$JXMlcw=="(i8* %fmt, i8* %p0, i8* %p1) {
entry:
	%0 = call i32 (i8*, ...) @printf(i8* %fmt, i8* %p0, i8* %p1)
	ret void
}

define void @main.foo(void (i8*)* %cb) {
entry:
	call void %cb(i8* getelementptr ([4 x i8], [4 x i8]* @1, i32 0, i32 0))
	ret void
}

define void @main.0(i8* %m2) {
entry:
	call void @main.bar(i8* getelementptr ([3 x i8], [3 x i8]* @2, i32 0, i32 0), i8* %m2)
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test8CurryCodegen(t *testing.T) {
	input := `
foo: (n:@str, s:@str){
	@printf("The winning number for %s is %s\n", s, n)
}
bar: foo("42")
baz: foo("43", "Bob")
bar("Alice")
baz()

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [33 x i8] c"The winning number for %s is %s\0A\00"
@1 = private unnamed_addr constant [3 x i8] c"42\00"
@2 = private unnamed_addr constant [3 x i8] c"43\00"
@3 = private unnamed_addr constant [4 x i8] c"Bob\00"
@4 = private unnamed_addr constant [6 x i8] c"Alice\00"

define i32 @main() {
entry:
	call void @main.bar(i8* getelementptr ([6 x i8], [6 x i8]* @4, i32 0, i32 0))
	call void @main.baz()
	ret i32 0
}

define void @main.foo(i8* %n, i8* %s) {
entry:
	call void @"printf$JXMlcw=="(i8* getelementptr ([33 x i8], [33 x i8]* @0, i32 0, i32 0), i8* %s, i8* %n)
	ret void
}

define void @main.bar(i8* %s) {
entry:
	call void @main.foo(i8* getelementptr ([3 x i8], [3 x i8]* @1, i32 0, i32 0), i8* %s)
	ret void
}

define void @main.baz() {
entry:
	call void @main.foo(i8* getelementptr ([3 x i8], [3 x i8]* @2, i32 0, i32 0), i8* getelementptr ([4 x i8], [4 x i8]* @3, i32 0, i32 0))
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test9Curry2Codegen(t *testing.T) {
	input := `
bar: (a:@str, b:@str, c:@str) {
    @printf("a: %s, b: %s, c: %s\n", a, b, c)
}
foo: (cb:(@str)){
    cb("c")
}
foo(bar("a", "b"))

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [21 x i8] c"a: %s, b: %s, c: %s\0A\00"
@1 = private unnamed_addr constant [2 x i8] c"c\00"
@2 = private unnamed_addr constant [2 x i8] c"a\00"
@3 = private unnamed_addr constant [2 x i8] c"b\00"

define i32 @main() {
entry:
	call void @main.foo(void (i8*)* @main.0)
	ret i32 0
}

define void @main.bar(i8* %a, i8* %b, i8* %c) {
entry:
	call void @printf$JXMlcyVz(i8* getelementptr ([21 x i8], [21 x i8]* @0, i32 0, i32 0), i8* %a, i8* %b, i8* %c)
	ret void
}

define void @main.foo(void (i8*)* %cb) {
entry:
	call void %cb(i8* getelementptr ([2 x i8], [2 x i8]* @1, i32 0, i32 0))
	ret void
}

define void @main.0(i8* %c) {
entry:
	call void @main.bar(i8* getelementptr ([2 x i8], [2 x i8]* @2, i32 0, i32 0), i8* getelementptr ([2 x i8], [2 x i8]* @3, i32 0, i32 0), i8* %c)
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}

func Test9Curry3Codegen(t *testing.T) {
	input := `
foo: (cb: ()){
    cb()
}
bar: (name: @str) {
    @printf("Hello, %s!\n", name)
}
foo(bar("John"))

`
	actual, err := GenerateIR(strings.NewReader(input))
	require.NoError(t, err)
	expected := `
@0 = private unnamed_addr constant [12 x i8] c"Hello, %s!\0A\00"
@1 = private unnamed_addr constant [5 x i8] c"John\00"

define i32 @main() {
entry:
	call void @main.foo(void ()* @main.0)
	ret i32 0
}

define void @main.foo(void ()* %cb) {
entry:
	call void %cb()
	ret void
}

define void @main.bar(i8* %name) {
entry:
	call void @"printf$JXM="(i8* getelementptr ([12 x i8], [12 x i8]* @0, i32 0, i32 0), i8* %name)
	ret void
}

define void @main.0() {
entry:
	call void @main.bar(i8* getelementptr ([5 x i8], [5 x i8]* @1, i32 0, i32 0))
	ret void
}

`
	require.Equal(t, strings.TrimSpace(expected), strings.TrimSpace(actual), fmt.Sprintf("got: %v", actual))
}
