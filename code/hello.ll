@x = private unnamed_addr constant [6 x i8] c"World\00"
@0 = private unnamed_addr constant [10 x i8] c"Hello %s\0A\00"

define i32 @main() {
entry:
	call void @"printf$JXM="(i8* getelementptr ([10 x i8], [10 x i8]* @0, i32 0, i32 0), [6 x i8]* @x)
	ret i32 0
}

define void @"printf$JXM="(i8* %fmt, i8* %p0) {
entry:
	%0 = call i32 (i8*, ...) @printf(i8* %fmt, i8* %p0)
	ret void
}

declare i32 @printf(i8* %fmt, ...)
