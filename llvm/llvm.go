package llvm

import (
	"encoding/base64"
	"fmt"
	"io"
	"strconv"
	"strings"

	"github.com/llir/llvm/ir"
	"github.com/llir/llvm/ir/constant"
	"github.com/llir/llvm/ir/enum"
	"github.com/llir/llvm/ir/types"
	"github.com/llir/llvm/ir/value"
	"github.com/rgolang/rgo/ast"
	"github.com/rgolang/rgo/lex"
	"github.com/rgolang/rgo/libcutils"
	"github.com/rgolang/rgo/omap"
)

// TODO: Library code should provide two variants of a public function that accepts at least one callback, one with a ctx param and one without appended as the last or first param and the callback will also have it appended as the last or first param, the user must use only one variant at all times to avoid having the function get duplicated in the binary.
// TODO: Internally the library code could use a single dispatch function where the external public functions are defined in a special entrypoint file and actually call the dispatch one, the dispatch function accepts max num of params (from all of the functions that it dispatches) where the last ones are set to nil when not used, rgo could internally only use the dispatch function.

var zero = constant.NewInt(types.I32, 0)
var builtin = make(map[string]any)

func GenerateIR(input io.ReadSeeker) (string, error) {
	lexer := lex.New(input)
	parser := ast.New(lexer)

	tree, err := parser.Parse()
	if err != nil {
		return "", fmt.Errorf("error parsing ast: %w", err)
	}
	mod, err := ToIR(tree)
	if err != nil {
		return "", fmt.Errorf("error generating IR: %w", err)
	}
	return mod.String(), nil
}

func ToIR(nodes []ast.Node) (*ir.Module, error) {
	module := ir.NewModule()
	ret := constant.NewInt(types.I32, 0)
	entry := ir.NewBlock("entry") // TODO: Rename entry to 0?
	fnc := module.NewFunc("main", ret.Type())
	ctx := newContext("main", module, fnc, entry)
	err := handleBody(ctx, nodes, ret)
	if err != nil {
		return nil, fmt.Errorf("error converting root nodes to IR: %w", err)
	}
	return module, nil
}

type Context struct { // TODO: Normalize everything to be a function, this means that a function can hold own params and a LLVMString method
	c     int
	id    string
	Mod   *ir.Module
	block *ir.Block
	inner *omap.Map[string, any]
	outer *omap.Map[string, any]
	// we can use names in inner/outer to figure out if it's the same param being passed as the closure env
	// TODO: it doesn't matter really, just the amount matters, all params can be ptr type (even ints)
}

func getParamsTypes(ctx *Context, node any) ([]types.Type, error) {
	if f, ok := node.(*ir.Func); ok {
		types := make([]types.Type, len(f.Params))
		for i := range f.Params {
			types[i] = f.Params[i].Typ
		}
		return types, nil
	}

	paramType, ok := node.(types.Type)
	if !ok {
		return nil, fmt.Errorf("node is not a param: %T", node)
	}

	ptr, ok := paramType.(*types.PointerType)
	if !ok {
		return nil, fmt.Errorf("not pointer type: %T", node)
	}

	if f, ok := ptr.ElemType.(*types.FuncType); ok {
		return f.Params, nil
	}

	return nil, fmt.Errorf("getParams elem type: %T", ptr.ElemType)
}

func handleBuiltIn(ctx *Context, name string, apply *ast.Apply) (any, bool) {
	// TODO: This needs improving and needs a lot more power, for example it should be able to return multiple instructions
	id := name
	comptimeArgs := map[int]any{}
	if name == "@prompt" {
		for i, arg := range apply.Arguments {
			switch a := arg.(type) {
			case *ast.StringLiteral:
				id += "$" + base64.StdEncoding.EncodeToString([]byte(a.Value))
				comptimeArgs[i] = a.Value
			case *ast.IntLiteral:
				id += "$" + strconv.Itoa(a.Value)
				comptimeArgs[i] = a.Value
			}
		}
	}

	if name == "@printf" {
		// TODO: fix the printf bug where no params ignores newline
		arg1 := apply.Arguments[0]
		fmtStr, ok := arg1.(*ast.StringLiteral)
		if !ok {
			panic("@printf first format argument must be a compile time string literal") // TODO: Remove panic
		}
		comptimeArgs[0] = fmtStr.Value

		specs, err := libcutils.ParsePrintfFmt(fmtStr.Value)
		if !ok {
			panic(fmt.Sprintf("error parsing printf format string %q: %s", fmtStr.Value, err)) // TODO: Remove panic
		}
		sb := strings.Builder{}
		for i := range specs {
			sb.WriteString(specs[i].Original)
		}

		id += "$" + base64.StdEncoding.EncodeToString([]byte(sb.String()))
		if _, ok := builtin[id]; ok {
			return builtin[id], ok
		}
	}

	if _, ok := builtin[id]; ok {
		return builtin[id], ok
	}

	mod := ctx.Mod

	// TODO: extern is not optimal
	switch name {
	case "@str":
		builtin[id] = types.I8Ptr
	case "@int":
		builtin[id] = types.I32
	case "@float":
		builtin[id] = types.Float
	case "@ieq", "@igt":
		x := ir.NewParam("x", types.I32)
		y := ir.NewParam("y", types.I32)
		cbtrue := ir.NewParam("true", types.NewPointer(types.NewFunc(types.Void)))
		cbfalse := ir.NewParam("false", types.NewPointer(types.NewFunc(types.Void)))

		f := mod.NewFunc("builtin."+strings.TrimLeft(name, "@"), types.Void, x, y, cbtrue, cbfalse)
		entry := f.NewBlock("entry")
		iftrue := f.NewBlock("iftrue")
		iftrue.NewCall(cbtrue)
		iftrue.NewRet(nil)
		iffalse := f.NewBlock("iffalse")
		iffalse.NewCall(cbfalse)
		iffalse.NewRet(nil)

		var cmp *ir.InstICmp
		switch name {
		case "@ieq":
			cmp = entry.NewICmp(enum.IPredEQ, x, y)
			entry.NewCondBr(cmp, iftrue, iffalse)
		case "@igt":
			cmp = entry.NewICmp(enum.IPredSGT, x, y)
			entry.NewCondBr(cmp, iftrue, iffalse)
		default:
			panic("unknown comparison: " + name)
		}
		entry.NewCondBr(cmp, iftrue, iffalse)

		builtin[id] = f
	case "@add":
		ok := ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I32)))
		x := ir.NewParam("x", types.I32)
		y := ir.NewParam("y", types.I32)
		f := mod.NewFunc("builtin.add", types.Void, x, y, ok)
		entry := f.NewBlock("entry")
		res := entry.NewAdd(x, y)
		entry.NewCall(ok, res)
		entry.NewRet(nil)
		builtin[id] = f
	case "@mul":
		ok := ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I32)))
		x := ir.NewParam("x", types.I32)
		y := ir.NewParam("y", types.I32)
		f := mod.NewFunc("builtin.mul", types.Void, x, y, ok)
		entry := f.NewBlock("entry")
		res := entry.NewMul(x, y)
		entry.NewCall(ok, res)
		entry.NewRet(nil)
		builtin[id] = f
	case "@unsafe.libc.atoi":
		in := ir.NewParam("in", types.I8Ptr)
		ok := ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I32)))
		f := mod.NewFunc("unsafe.libc.atoi", types.Void, in, ok)
		entry := f.NewBlock("entry")
		atoi := mod.NewFunc("atoi", types.I32, in)
		res := entry.NewCall(atoi, in) // TODO: Use in.Param() ?
		// convert i32 res to i64
		// res64 := entry.NewZExt(res, types.I32)
		entry.NewCall(ok, res)
		entry.NewRet(nil)
		builtin[id] = f
	case "@unsafe.libc.puts":
		str := ir.NewParam("str", types.I8Ptr)
		ok := ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I32)))
		f := mod.NewFunc("unsafe.libc.puts", types.Void, str, ok)
		entry := f.NewBlock("entry")
		puts := mod.NewFunc("puts", types.I32, str)
		res := entry.NewCall(puts, str)
		// res64 := entry.NewZExt(res, types.I32)
		entry.NewCall(ok, res)
		entry.NewRet(nil)
		builtin[id] = f
	case "@unsafe.libc.printf":
		fmt := ir.NewParam("fmt", types.I8Ptr)
		f := mod.NewFunc("printf", types.I32, fmt)
		f.Sig.Variadic = true
		builtin[id] = f
	case "@unsafe.libc.scanf":
		fmt := ir.NewParam("fmt", types.I8Ptr)
		f := mod.NewFunc("__isoc99_scanf", types.I32, fmt)
		f.Sig.Variadic = true
		builtin[id] = f
	case "@unsafe.libc.getchar":
		f := mod.NewFunc("getchar", types.I32)
		builtin[id] = f
	case "@unsafe.libc.fgets":
		fmt := ir.NewParam("fmt", types.I8Ptr)
		n := ir.NewParam("n", types.I32)
		stream := ir.NewParam("stream", types.I8Ptr)
		f := mod.NewFunc("fgets", types.I8Ptr, fmt, n, stream)
		builtin[id] = f
	case "@printf":
		fmtStr, ok := comptimeArgs[0].(string)
		if !ok {
			panic("@printf first format argument must be a compile time string literal") // TODO: Remove panic
		}
		specs, err := libcutils.ParsePrintfFmt(fmtStr)
		if !ok {
			panic(fmt.Sprintf("error parsing printf format string %q: %s", fmtStr, err)) // TODO: Remove panic
		}
		fmtS := ir.NewParam("fmt", types.I8Ptr)
		params := []*ir.Param{fmtS}
		params2 := []*ir.Param{fmtS}
		args := []value.Value{fmtS}

		for i, spec := range specs {
			var t2 types.Type
			switch spec.Specifier {
			case "i", "d":
				t2 = types.I32
			case "s":
				t2 = types.I8Ptr
			default:
				panic(fmt.Sprintf("specifier not supported in printf: %q", spec.Specifier)) // TODO: Remove panic
			}
			p := ir.NewParam("p"+strconv.Itoa(i), t2)
			params = append(params, p)
			params2 = append(params2, p)
			args = append(args, p)
		}
		nm := strings.TrimLeft(id, "@")
		f := mod.NewFunc(nm, types.Void, params2...)

		printfAny, ok := handleBuiltIn(ctx, "@unsafe.libc.printf", nil)
		if !ok {
			panic("@unsafe.libc.printf not found")
		}
		printf := printfAny.(*ir.Func)

		entry := f.NewBlock("entry")
		entry.NewCall(printf, args...) // TODO: In prod ignore error since it's compile time checked already, in dev panic
		entry.NewRet(nil)

		builtin[id] = f
		return builtin[id], true
	case "@prompt":
		limit, ok := comptimeArgs[0].(int)
		if !ok {
			panic("@prompt limit must be an int") // TODO: Remove panic
		}
		f := declarePrompt(ctx, id, limit)
		builtin[id] = f
		return f, true
	case "@std":
		ctx.add("string", types.I8Ptr)
		ctx.add("float", types.Float)
		ctx.add("int", types.I32)
		f := mod.NewFunc("std", types.Void)
		entry := f.NewBlock("entry")
		entry.NewRet(nil)
		builtin[id] = f
	}
	// TODO: disallow dots in function definition name

	// TODO: need to look into your platfrom stdio.h header to figure it out what symbol/declaration to use for stdio, so it links correctly with C runtime library you will use

	// Import external functions or definitions (TODO: Parse a header file for this)

	// TODO: Should handle the error returned by `puts`

	// case "FILE":// TODO: Not OS consistent, need time to implement
	// 	fileType := types.NewStruct() // Empty struct, treated as opaque
	// 	fileType.Opaque = true
	// 	FILE := ctx.Module.NewTypeDef("FILE", fileType)
	// 	ctx.Extern["FILE"] = FILE
	// 	return FILE
	// case "@unsafe.stdout": // TODO: Not OS consistent, need time to implement
	// 	fileType, ok := extern(ctx, "FILE").(types.Type)
	// 	if !ok {
	// 		panic("FILE is not the right type for stdout")
	// 	}
	// 	stdout := ctx.Module.NewGlobal("stdout", fileType)
	// 	stdout.Linkage = enum.LinkageExternal
	// 	ctx.Extern[name] = stdout
	// 	return stdout
	// case "@unsafe.stderr":// TODO: Not OS consistent, need time to implement
	// 	fileType, ok := extern(ctx, "FILE").(types.Type)
	// 	if !ok {
	// 		panic("FILE is not the right type for stderr")
	// 	}
	// 	stderr := ctx.Module.NewGlobal("stderr", fileType)
	// 	stderr.Linkage = enum.LinkageExternal
	// 	ctx.Extern[name] = stderr
	// 	return stderr
	// case "@unsafe.fprintf": // TODO: Not OS consistent, need time to implement
	// 	fileType, ok := extern(ctx, "FILE").(types.Type)
	// 	if !ok {
	// 		panic("FILE is not the right type for fprintf")
	// 	}
	// 	fprintf := ctx.Module.NewFunc("fprintf", types.I32, ir.NewParam("", types.NewPointer(fileType)), ir.NewParam("", types.I8Ptr))
	// 	fprintf.Sig.Variadic = true
	// 	ctx.Extern[name] = fprintf
	// 	return fprintf

	if _, ok := builtin[id]; ok {
		return builtin[id], ok
	}
	return nil, false
}

func newContext(id string, mod *ir.Module, fnc *ir.Func, entry *ir.Block) *Context {
	if entry != nil {
		entry.Parent = fnc
		fnc.Blocks = append(fnc.Blocks, entry)
	}
	return &Context{
		id:    id,
		Mod:   mod,
		block: entry,
		inner: omap.New[string, any](),
		outer: omap.New[string, any](),
	}
}

func (ctx *Context) new(name string, mod *ir.Module, fnc *ir.Func, entry *ir.Block) *Context {
	id := strings.Builder{}
	id.WriteString(ctx.id)
	id.WriteString(".")
	if name == "" {
		id.WriteString(strconv.Itoa(ctx.c))
		ctx.c++
	} else {
		id.WriteString(name)
	}

	newCtx := newContext(id.String(), mod, fnc, entry)

	// Copy outer frame inner to this frame outer
	ctx.outer.Each(func(k string, v any) {
		newCtx.outer.Set(k, v)
	})
	ctx.inner.Each(func(k string, v any) {
		newCtx.outer.Set(k, v)
	})

	if fnc != nil {
		fnc.SetName(id.String())

		// Copy params to inner (overwriting any outer values, shadowing)
		for _, p := range fnc.Params {
			if p.Name() == "" {
				panic("param has no name")
			}
			newCtx.inner.Set(p.Name(), p)
		}
	}

	// Replace every outer value that is not constant to be a global
	newCtx.outer.Each(func(k string, v any) {
		// if already part of inner, skip
		if _, ok := newCtx.inner.Map[k]; ok {
			return
		}
		switch n := v.(type) {
		case *ir.Func:
		case *ir.Global:
		case *constant.ExprGetElementPtr:
		case *types.FuncType:
		case *types.FloatType:
		case *types.IntType:
		case *types.PointerType:
			switch t := n.ElemType.(type) {
			case *types.FloatType:
			case *types.IntType:
			case *types.FuncType:
			default:
				panic(fmt.Sprintf("outer ptr type not supported: %s %T", k, t))
			}
		case *ir.InstPtrToInt: // TODO: Remove
			src, ok := n.From.(*ir.Global)
			if !ok {
				panic(fmt.Sprintf("outer int load type not supported: %s %T", k, n.From))
			}
			i := newCtx.block.NewPtrToInt(src, n.Type())
			i.LocalName = k
			newCtx.outer.Set(k, i)
		case *ir.InstLoad:
			if _, ok := n.Src.(*ir.Global); !ok {
				panic(fmt.Sprintf("outer load type not supported: %s %T", k, n.Src))
			}
			load := newCtx.block.NewLoad(n.Type(), n.Src) // n.Src is the global, this forwards it
			load.LocalName = k
			newCtx.outer.Set(k, load)
		case *ir.Param:
			// This is a hack around the llvm limitation of not being able to use the stack outside of the function
			// TODO: Recycle these globals based on max amount of needed globals instead of creating on demand
			g := mod.NewGlobalDef("", constant.NewNull(types.NewPointer(nil))) // TODO: Create a pull request to merge opaque pointer support into llir/llvm
			var v value.Value = n
			ctx.block.NewStore(v, g)
			load := newCtx.block.NewLoad(v.Type(), g)
			load.LocalName = k
			newCtx.outer.Set(k, load)
		default:
			panic(fmt.Sprintf("outer type not supported: %q with type %T, called from %q", k, v, ctx.id))
		}
	})
	return newCtx
}

func (f *Context) add(name string, fn any) error {
	if name == "" {
		return fmt.Errorf("adding definition for label %q: label name cannot be empty", f.id)
	}
	if _, ok := f.inner.Map[name]; ok {
		return fmt.Errorf("add definition: label already exists: %v", name)
	}
	f.inner.Set(name, fn)
	return nil
}

func (f *Context) get(name string) (any, error) {
	v, ok := f.inner.Get(name)
	if !ok {
		v, ok = f.outer.Get(name)
		if !ok {
			return nil, fmt.Errorf("label %q not found in this scope", name)
		}
	}
	return v, nil
}

func handleParam(ctx *Context, t *ast.Type) (*ir.Param, error) {
	if t == nil {
		return nil, fmt.Errorf("param node == nil")
	}

	// Handle basic types
	if t.Value != "" {
		if builtin, ok := handleBuiltIn(ctx, t.Value, nil); ok {
			if typ, ok := builtin.(types.Type); ok {
				return ir.NewParam(t.Name, typ), nil
			}
			return nil, fmt.Errorf("built-in type is not a type: %q %T", t.Value, builtin)
		}
		x, err := ctx.get(t.Value)
		if err != nil {
			return nil, fmt.Errorf("cannot find type: %w\n%s", err, ast.DebugPrintLocation(t))
		}
		if typ, ok := x.(types.Type); ok {
			return ir.NewParam(t.Name, typ), nil
		}
		return nil, fmt.Errorf("param type is not a type: %q %T", t.Value, x)
	}

	// Handle function pointer
	funcType, err := handleType(ctx, t)
	if err != nil {
		return nil, fmt.Errorf("failed to handle param type: %w", err)
	}
	return ir.NewParam(t.Name, funcType), nil
}

func handleType(ctx *Context, t *ast.Type) (types.Type, error) {
	funcType := types.NewFunc(types.Void)
	funcCopy := *funcType
	innerTypes := make([]types.Type, 0)
	if t.Name != "" {
		// llvm doesn't support recursive types that aren't structures, so just make it into an opaque pointer
		ctx = ctx.new(t.Name, ctx.Mod, nil, nil)
		ptr := types.NewPointer(nil)
		ctx.add(t.Name, ptr) // support recursive types
	}
	for _, t := range t.Values {
		p, err := handleParam(ctx, t)
		if err != nil {
			return nil, fmt.Errorf("failed to handle function type: %w", err)
		}
		innerTypes = append(innerTypes, p.Typ)
	}
	funcType.Params = innerTypes
	funcCopy.Params = innerTypes
	return types.NewPointer(funcType), nil
}

func handleNode(ctx *Context, node ast.Node, paramType types.Type) (value.Value, error) {
	var err error
	var callee value.Value
	// Remember it internally
	switch n := node.(type) { // TODO: This would have been the Declaration AST node
	case *ast.Function:
		callee, err = handleFunctionNode(ctx, n)
		if err != nil {
			return nil, fmt.Errorf("error handling function: %w", err)
		}
	case *ast.Apply:
		callee, err = handleApplyNode(ctx, n)
		if err != nil {
			return nil, fmt.Errorf("error handling apply: %w", err)
		}
	case *ast.Label:
		// TODO: Allow variadic builtins here
		a, err := ctx.get(n.Of) // TODO: this is used identically in two places
		if err != nil {
			return nil, fmt.Errorf("error finding label: %w\n%s", err, ast.DebugPrintLocation(node))
		}
		if v, ok := a.(value.Value); ok {
			return v, nil
		}
		return nil, fmt.Errorf("found label %q, but is not a value: %w\n%s", n.Of, err, ast.DebugPrintLocation(node))
	case *ast.IntLiteral:
		// TODO: slightly duplicated
		v := constant.NewInt(types.I32, int64(n.Value)) // TODO: Assuming 64-bit integers
		return v, nil
	case *ast.StringLiteral:
		// TODO: slightly duplicated
		v := ctx.newConstString(n.Name, n.Value)
		return v, nil
	default:
		return nil, fmt.Errorf("unsupported node type: %T", node)
	}

	if paramType != nil {
		// The apply is adding more params than the callback param supports
		callbackParams, err := getParamsTypes(ctx, paramType)
		if err != nil {
			return nil, fmt.Errorf("error getting callback params: %w", err)
		}
		params, err := getParamsTypes(ctx, callee)
		if err != nil {
			return nil, fmt.Errorf("error getting apply params: %w", err)
		}
		if len(callbackParams) < len(params) {
			return nil, fmt.Errorf("adding more arguments than the callback %q expects, %q expected %d arguments, got %d", callee.Ident(), paramType.Name(), len(callbackParams), len(params))
		}
	}
	return callee, nil
}

func handleFunctionNode(ctx *Context, n *ast.Function) (*ir.Func, error) {
	name := n.Name
	if name == "" {
		name = strconv.Itoa(ctx.c)
		ctx.c++ // TODO: is this duplicate in the ctx?
	}

	// handle params
	params := make([]*ir.Param, len(n.Params))
	for i, p := range n.Params {
		p, err := handleParam(ctx, p)
		if err != nil {
			return nil, fmt.Errorf("[%d]: func: %w", i, err)
		}
		params[i] = p
	}

	fnc := ctx.Mod.NewFunc(name, types.Void, params...)
	newCtx := ctx.new(name, ctx.Mod, fnc, ir.NewBlock("entry"))
	err := newCtx.add(name, fnc)
	if err != nil {
		return nil, fmt.Errorf("add fn ref: %w", err)
	}

	err = handleBody(newCtx, n.Body, nil)
	if err != nil {
		return nil, fmt.Errorf("error handling function body for function %q: %w", n.Name, err)
	}
	return fnc, nil
}

func handleApplyNode(ctx *Context, n *ast.Apply) (value.Value, error) {
	var callee *ir.Func
	switch f := n.Callee.(type) {
	case *ast.Function:
		fn, err := handleFunctionNode(ctx, f)
		if err != nil {
			return nil, fmt.Errorf("error handling anon function %q: %w", n.Name, err)
		}
		callee = fn
	case *ast.Label:
		var err error
		v, ok := handleBuiltIn(ctx, f.Of, n) // TODO: handle this in handleCallable
		if !ok {
			v, err = ctx.get(f.Of)
			if err != nil {
				return nil, fmt.Errorf("error finding label: %w\n%s", err, ast.DebugPrintLocation(n))
			}
		}
		// TODO: Support applying a param
		if callee, ok = v.(*ir.Func); !ok {
			return nil, fmt.Errorf("label found, but is not a function: %q", f.Of)
		}
	default:
		// TODO: handle other types of apply
		return nil, fmt.Errorf("unsupported callee type: %T", n.Callee)
	}

	calleeParams, err := getParamsTypes(ctx, callee)
	if err != nil {
		return nil, fmt.Errorf("error getting callee params: %w", err)
	}

	// The apply is adding more params than the callee supports
	if len(calleeParams) < len(n.Arguments) {
		return nil, fmt.Errorf("%q expected params %d, got %d", callee.Ident(), len(calleeParams), len(n.Arguments))
	}

	args := make([]value.Value, 0, len(n.Arguments))
	for i, arg := range n.Arguments {
		v, err := handleNode(ctx, arg, calleeParams[i])
		if err != nil {
			return nil, fmt.Errorf("[%d] error converting argument: %w", i, err)
		}
		args = append(args, v)
	}

	params := make([]*ir.Param, len(calleeParams[len(args):]))
	for i, p := range callee.Params[len(args):] {
		params[i] = p
		args = append(args, p)
	}
	fnc := ctx.Mod.NewFunc(n.Name, types.Void, params...)
	apply := ctx.new(n.Name, ctx.Mod, fnc, ir.NewBlock("entry")) // call frame

	// Call the callee from inside the apply helper
	apply.block.NewCall(callee, args...) // TODO: This might not be enough args
	apply.block.NewRet(nil)
	return fnc, nil
}

func handleBody(ctx *Context, nodes []ast.Node, ret value.Value) error {
	for i, node := range nodes {
		// TODO: Now that Label() is available, could simply use handleNode (for strings could do the pointer outside of handleNode? or get the thing the value points at?)
		// A declaration can either a function, a constant or an alias to a function, constant or parameter.
		// A declaration can never be a variable.
		switch n := node.(type) { // TODO: This would have been the Declaration AST node
		case *ast.Function:
			callee, err := handleFunctionNode(ctx, n)
			if err != nil {
				return fmt.Errorf("[%d] in body: error handling function %q: %w", i, n.Name, err)
			}
			if n.Name == "" {
				ctx.block.NewCall(callee)
			} else {
				err = ctx.add(n.Name, callee)
				if err != nil {
					return fmt.Errorf("add fn ref: %w", err)
				}
			}
		case *ast.Apply:
			if n.Name == "" {
				var callee value.Value
				if label, ok := n.Callee.(*ast.Label); ok {
					// TODO: could return a function, since everything is a function
					if builtin, ok := handleBuiltIn(ctx, label.Of, n); ok {
						if callee, ok = builtin.(value.Value); !ok {
							return fmt.Errorf("handle builtin call: %v", label.Of)
						}
					}
				}
				if callee == nil {
					// TODO: Change this to support any type
					var err error
					callee, err = handleNode(ctx, n.Callee, nil)
					if err != nil {
						return fmt.Errorf("handle call: %w", err)
					}
				}

				// Just call the unsafe functions directly, hence they are unsafe
				args := make([]value.Value, 0, len(n.Arguments))
				for i, arg := range n.Arguments {
					v, err := handleNode(ctx, arg, nil)
					if err != nil {
						return fmt.Errorf("[%d] error converting argument: %w", i, err)
					}
					args = append(args, v)
				}
				ctx.block.NewCall(callee, args...)
			} else {
				callee, err := handleApplyNode(ctx, n)
				if err != nil {
					return fmt.Errorf("[%d] in body: error handling apply: %w", i, err)
				}
				err = ctx.add(n.Name, callee)
				if err != nil {
					return fmt.Errorf("add fn ref: %w", err)
				}
			}
		case *ast.IntLiteral:
			// TODO: Duplicate code in handleNode
			if n.Name == "" {
				return fmt.Errorf("calling an int literal statement is not supported")
			}

			v := constant.NewInt(types.I32, int64(n.Value))
			err := ctx.add(n.Name, v) // TODO: check for empty
			if err != nil {
				return fmt.Errorf("add int ref: %w", err)
			}
		case *ast.StringLiteral:
			// TODO: Duplicate code in handleNode
			if n.Name == "" {
				return fmt.Errorf("anonymous string literal statement is not supported")
			}
			v := ctx.newConstString(n.Name, n.Value)
			err := ctx.add(n.Name, v) // TODO: check for empty
			if err != nil {
				return fmt.Errorf("add str ref: %w", err)
			}
		case *ast.Type:
			if n.Name == "" {
				return fmt.Errorf("anonymous type statement is not supported\n%v", ast.DebugPrintLocation(n))
			}
			funcType, err := handleType(ctx, n)
			if err != nil {
				return fmt.Errorf("handle type declaration: %w", err)
			}
			err = ctx.add(n.Name, funcType) // TODO: check for empty
			if err != nil {
				return fmt.Errorf("add str ref: %w", err)
			}
		default:
			return fmt.Errorf("unsupported declaration type: %T", node)
		}
	}
	ctx.block.NewRet(ret)
	return nil
}

func (ctx *Context) newConstString(name string, value string) *constant.ExprGetElementPtr {
	mod := ctx.Mod
	// Convert the string literal to an LLVM IR constant array of characters
	// Note: In LLVM, strings are typically null-terminated
	strVal := value + "\x00"
	strType := types.NewArray(uint64(len(strVal)), types.I8)

	// Create a constant for the string
	constStr := ir.NewGlobalDef(name, constant.NewCharArrayFromString(strVal))
	constStr.Typ = types.NewPointer(strType)
	constStr.Immutable = true                          // Mark as constant
	constStr.Linkage = enum.LinkagePrivate             // For optimization
	constStr.UnnamedAddr = enum.UnnamedAddrUnnamedAddr // For optimization
	mod.Globals = append(mod.Globals, constStr)
	return constant.NewGetElementPtr(constStr.Typ.ElemType, constStr, zero, zero)
}

func declarePrompt(ctx *Context, id string, limit int) *ir.Func {
	name := "builtin." + strings.TrimLeft(id, "@")

	scanfAny, isFunc := handleBuiltIn(ctx, "@unsafe.libc.scanf", nil)
	if !isFunc {
		panic("@unsafe.libc.scanf not found")
	}

	scanf, isCallable := scanfAny.(value.Value)
	if !isCallable {
		panic("@unsafe.libc.scanf not a function")
	}

	// TODO: Maybe this can be implemented in rgo?
	// TODO: @scanstack(@stdin, limit, (stackdata: @str){}) // TODO: Byte array type
	// TODO: @scanheap(@stdin, limit, (heapdata: @str){}) // TODO: Byte array type
	limitParam := ir.NewParam("limit", types.NewPointer(types.NewFunc(types.Void, types.I8Ptr)))
	ok := ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I8Ptr)))
	prompt := ctx.Mod.NewFunc(name, types.Void, limitParam, ok)
	entry := prompt.NewBlock("entry")

	// Allocate a buffer with size equal to limit + 1 (for null terminator).
	bufferType := types.NewArray(uint64(limit+1), types.I8)
	inputBuffer := entry.NewAlloca(bufferType)

	formatStrPtr := ctx.newConstString("builtin.prompt$"+strconv.Itoa(limit)+".format", "%"+strconv.Itoa(limit)+"s")
	bufferPtr := entry.NewGetElementPtr(bufferType, inputBuffer, constant.NewInt(types.I32, 0), constant.NewInt(types.I32, 0))
	entry.NewCall(scanf, formatStrPtr, bufferPtr)

	entry.NewCall(ok, bufferPtr)
	entry.NewRet(nil)

	return prompt
}
