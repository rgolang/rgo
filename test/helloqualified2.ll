@x = private unnamed_addr constant [14 x i8] c"Hello world!\0A\00"

define i32 @main() {
entry:
	call void @main.foo()
	ret i32 0
}

define void @main.foo() {
entry:
	call void @unsafe.libc.puts([14 x i8]* @x, void (i64)* @main.foo.0)
	ret void
}

define void @unsafe.libc.puts(i8* %str, void (i64)* %ok) {
entry:
	%0 = call i32 @puts(i8* %str)
	%1 = zext i32 %0 to i64
	call void %ok(i64 %1)
	ret void
}

declare i32 @puts(i8* %str)

define void @main.foo.0(i64 %code) {
entry:
	ret void
}
