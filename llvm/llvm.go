package llvm

import (
	"bufio"
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
)

// TODO: Library code should provide two variants of a public function that accepts at least one callback, one with a ctx param and one without appended as the last or first param and the callback will also have it appended as the last or first param, the user must use only one variant at all times to avoid having the function get duplicated in the binary.
// TODO: Internally the library code could use a single dispatch function where the external public functions are defined in a special entrypoint file and actually call the dispatch one, the dispatch function accepts max num of params (from all of the functions that it dispatches) where the last ones are set to nil when not used, rgo could internally only use the dispatch function.

var zero = constant.NewInt(types.I32, 0)
var builtin = make(map[string]any)

func GenerateIR(input io.Reader) (string, error) {
	reader := bufio.NewReader(input)
	lexer := lex.New(reader)
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
	ctx := newContext(module, "main", ret, nil, entry, false)
	err := handleBody(ctx, nodes, ret)
	if err != nil {
		return nil, fmt.Errorf("error converting root nodes to IR: %w", err)
	}
	return module, nil
}

type Context struct { // TODO: Normalize everything to be a function, this means that a function can hold own params and a LLVMString method
	c     int
	id    string
	Func  *ir.Func
	block *ir.Block
	inner map[string]any
	outer map[string]any
	// we can use names in inner/outer to figure out if it's the same param being passed as the closure env
	// TODO: it doesn't matter really, just the amount matters, all params can be ptr type (even ints)
}

func getParams(ctx *Context, node any) ([]*ir.Param, error) {
	if f, ok := node.(*ir.Func); ok {
		return f.Params, nil
	}

	param, ok := node.(*ir.Param)
	if !ok {
		return nil, fmt.Errorf("getParams: %T", node)
	}

	ptr, ok := param.Typ.(*types.PointerType)
	if !ok {
		return nil, fmt.Errorf("getParams: %T", node)
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
		arg1 := apply.Arguments[0]
		fmtStr, ok := arg1.(*ast.StringLiteral)
		if !ok {
			panic("@printf first format argument must be a compile time string literal") // TODO: Remove panic
		}
		comptimeArgs[0] = fmtStr.Value

		specs, err := libcutils.ParsePrintfFmt(fmtStr.Value)
		if !ok {
			panic(fmt.Sprintf("error parsing printf format string %q: %s", fmtStr, err)) // TODO: Remove panic
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

	mod := ctx.Func.Parent

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
		ctx.set("string", types.I8Ptr)
		ctx.set("float", types.Float)
		ctx.set("int", types.I32)
		f := mod.NewFunc("std", types.Void)
		entry := f.NewBlock("entry")
		entry.NewRet(nil)
		builtin[id] = f
	}
	// TODO: disallow dots in function definition name

	// TODO: You need to look into your platfrom stdio.h header to figure it out what symbol/declaration to use for stdio, so it links correctly with C runtime library you will use

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

func newContext(mod *ir.Module, id string, ret value.Value, params []*ir.Param, entry *ir.Block, isVariadic bool) *Context {
	var retType types.Type = types.Void
	if ret != nil {
		retType = ret.Type()
	}
	fnc := mod.NewFunc(id, retType, params...)
	fnc.Sig.Variadic = isVariadic
	if entry != nil {
		entry.Parent = fnc
		fnc.Blocks = append(fnc.Blocks, entry)
	}
	return &Context{
		id:    id,
		Func:  fnc,
		block: entry,
		inner: make(map[string]any),
		outer: make(map[string]any),
	}
}

func (ctx *Context) new(name string, params []*ir.Param, entry *ir.Block, isVariadic bool) *Context {
	id := strings.Builder{}
	id.WriteString(ctx.id)
	id.WriteString(".")
	if name == "" {
		id.WriteString(strconv.Itoa(ctx.c))
		ctx.c++
	} else {
		id.WriteString(name)
	}
	newCtx := newContext(ctx.Func.Parent, id.String(), nil, params, entry, isVariadic)

	// Copy outer frame inner to this frame outer
	for k, v := range ctx.outer {
		newCtx.outer[k] = v
	}
	for k, v := range ctx.inner {
		newCtx.outer[k] = v
	}

	// Copy params to inner (overwriting any outer values, shadowing)
	for _, p := range params {
		if p.Name() == "" {
			panic("param has no name")
		}
		newCtx.inner[p.Name()] = p
	}

	// Replace every outer value that is not constant to be a global
	for k, v := range newCtx.outer {
		// if already part of inner, skip
		if _, ok := newCtx.inner[k]; ok {
			continue
		}
		switch n := v.(type) {
		case *ir.Func:
		case *ir.Global:
		case *types.FloatType:
		case *types.IntType:
		case *types.PointerType:
			switch t := n.ElemType.(type) {
			case *types.FloatType:
			case *types.IntType:
			default:
				panic(fmt.Sprintf("outer ptr type not supported: %s %T", k, t))
			}
		case *ir.InstPtrToInt:
			src, ok := n.From.(*ir.Global)
			if !ok {
				panic(fmt.Sprintf("outer int load type not supported: %s %T", k, n.From))
			}
			i := newCtx.block.NewPtrToInt(src, n.Type())
			i.LocalName = k
			newCtx.outer[k] = i
		case *ir.InstLoad:
			if _, ok := n.Src.(*ir.Global); !ok {
				panic(fmt.Sprintf("outer load type not supported: %s %T", k, n.Src))
			}
			load := newCtx.block.NewLoad(n.Type(), n.Src) // n.Src is the global, this forwards it
			load.LocalName = k
			newCtx.outer[k] = load
		case *ir.Param:
			// This is a hack around the llvm limitation of not being able to use the stack outside of the function
			// TODO: Recycle these globals based on max amount of needed globals instead of creating on demand
			g := ctx.Func.Parent.NewGlobalDef("", constant.NewNull(types.NewPointer(nil))) // TODO: Create a pull request to merge opaque pointer support into llir/llvm
			var v value.Value = n
			isInt := n.Type().Equal(types.I32)
			if isInt {
				// a ptr is actually an int, this skips unnecessary allocations and transfers the int as a ptr
				v = ctx.block.NewIntToPtr(n, types.NewPointer(nil))
			}
			ctx.block.NewStore(v, g)
			if isInt {
				i := newCtx.block.NewPtrToInt(g, n.Type())
				i.LocalName = k
				v = i
			} else {
				load := newCtx.block.NewLoad(v.Type(), g)
				load.LocalName = k
				v = load
			}
			newCtx.outer[k] = v
		default:
			panic(fmt.Sprintf("outer type not supported: %q with type %T, called from %q", k, v, ctx.id))
		}
	}
	return newCtx
}

func (f *Context) set(name string, fn any) error {
	if name == "" {
		return fmt.Errorf("adding definition for label %q: label name cannot be empty", f.id)
	}
	if _, ok := f.inner[name]; ok {
		return fmt.Errorf("add definition: label already exists: %v", name)
	}
	f.inner[name] = fn
	return nil
}

func (f *Context) get(name string) (any, error) {
	v, ok := f.inner[name]
	if !ok {
		v, ok = f.outer[name]
		if !ok {
			return nil, fmt.Errorf("label %q not found in inner or outer scope", name)
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
			return nil, fmt.Errorf("failed to get type: %w", err)
		}
		if typ, ok := x.(types.Type); ok {
			return ir.NewParam(t.Name, typ), nil
		}
		return nil, fmt.Errorf("param type is not a type: %q %T", t.Value, x)
	}

	// Handle function pointer
	innerTypes := make([]types.Type, 0)
	for _, t := range t.Values {
		p, err := handleParam(ctx, t)
		if err != nil {
			return nil, fmt.Errorf("failed to handle function pointer: %w", err)
		}
		innerTypes = append(innerTypes, p.Typ)
	}
	return ir.NewParam(t.Name, types.NewPointer(types.NewFunc(types.Void, innerTypes...))), nil
}

func handleNode(ctx *Context, node ast.Node, param *ir.Param) (value.Value, error) {
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
			return nil, fmt.Errorf("error finding label: %w", err)
		}
		if v, ok := a.(value.Value); ok {
			return v, nil
		}
		return nil, fmt.Errorf("found label %q, but is not a value: %w", n.Of, err)
	case *ast.IntLiteral:
		// TODO: slightly duplicated
		v := constant.NewInt(types.I32, int64(n.Value)) // TODO: Assuming 64-bit integers
		return v, nil
	case *ast.StringLiteral:
		// TODO: slightly duplicated
		globalStr := ctx.newConstString(n.Name, n.Value)
		v := constant.NewGetElementPtr(globalStr.Typ.ElemType, globalStr, zero, zero)
		return v, nil
	default:
		return nil, fmt.Errorf("unsupported declaration type: %T", node)
	}

	if param != nil {
		// The apply is adding more params than the callback param supports
		callbackParams, err := getParams(ctx, param)
		if err != nil {
			return nil, fmt.Errorf("error getting callback params: %w", err)
		}
		params, err := getParams(ctx, callee)
		if err != nil {
			return nil, fmt.Errorf("error getting apply params: %w", err)
		}
		if len(callbackParams) < len(params) {
			return nil, fmt.Errorf("adding more arguments than the callback %q expects, %q expected %d arguments, got %d", callee.Ident(), param.Name(), len(callbackParams), len(params))
		}
	}
	return callee, nil
}

func handleFunctionNode(ctx *Context, n *ast.Function) (*ir.Func, error) {
	name := n.Name
	if name == "" {
		name = strconv.Itoa(ctx.c)
		ctx.c++
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

	fn := ctx.new(name, params, ir.NewBlock("entry"), false)
	err := handleBody(fn, n.Body, nil)
	if err != nil {
		return nil, fmt.Errorf("error handling function body for function %q: %w", n.Name, err)
	}
	return fn.Func, nil
}

func handleApplyNode(ctx *Context, n *ast.Apply) (value.Value, error) {
	var callee value.Value
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
				return nil, fmt.Errorf("error finding label: %w", err)
			}
		}
		if callee, ok = v.(value.Value); !ok {
			return nil, fmt.Errorf("label found, but is not value: %q", f.Of)
		}
	default:
		// TODO: handle other types of apply
		return nil, fmt.Errorf("unsupported callee type: %T", n.Callee)
	}

	calleeParams, err := getParams(ctx, callee)
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
	for i, p := range calleeParams[len(args):] {
		params[i] = p
		args = append(args, p)
	}
	apply := ctx.new(n.Name, params, ir.NewBlock("entry"), false) // call frame

	// Call the callee from inside the apply helper
	apply.block.NewCall(callee, args...) // TODO: This might not be enough args
	apply.block.NewRet(nil)
	return apply.Func, nil
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
				err = ctx.set(n.Name, callee)
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
				err = ctx.set(n.Name, callee)
				if err != nil {
					return fmt.Errorf("add fn ref: %w", err)
				}
			}
		case *ast.IntLiteral:
			// TODO: Duplicate code in handleNode
			if n.Name == "" {
				return fmt.Errorf("calling an int literal statement is not supported")
			}

			v := constant.NewInt(types.I32, int64(n.Value)) // TODO: Assuming 64-bit integers
			err := ctx.set(n.Name, v)                       // TODO: check for empty
			if err != nil {
				return fmt.Errorf("add int ref: %w", err)
			}
		case *ast.StringLiteral:
			// TODO: Duplicate code in handleNode
			if n.Name == "" {
				return fmt.Errorf("anonymous string literal statement is not supported")
			}
			globalStr := ctx.newConstString(n.Name, n.Value)
			err := ctx.set(n.Name, globalStr) // TODO: check for empty
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

func (ctx *Context) newConstString(name string, value string) *ir.Global {
	mod := ctx.Func.Parent
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
	return constStr
}

func declarePrompt(ctx *Context, id string, limit int) *ir.Func { // TODO: Could just use fscan with the limit inside prompt
	name := "builtin." + strings.TrimLeft(id, "@")
	// TODO: Limit should be a compile time param

	scanfAny, isFunc := handleBuiltIn(ctx, "@unsafe.libc.scanf", nil) // TODO: handle this in handleCallable
	if !isFunc {
		panic("@unsafe.libc.scanf not found")
	}

	scanf, isCallable := scanfAny.(value.Value)
	if !isCallable {
		panic("@unsafe.libc.scanf not a function")
	}

	// TODO: Maybe this can be implemented in rgo?
	// TODO: Maybe use scanf("%3s", string)?
	// limitParam := ir.NewParam("limit", types.I32) // TODO: Make it dynamic and safe? Compile time value?
	limitParam := ir.NewParam("limit", types.NewPointer(types.NewFunc(types.Void, types.I8Ptr)))
	ok := ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I8Ptr)))
	prompt := ctx.Func.Parent.NewFunc(name, types.Void, limitParam, ok)
	entry := prompt.NewBlock("entry")

	// Allocate a buffer with size equal to limit + 1 (for null terminator).
	bufferType := types.NewArray(uint64(limit+1), types.I8)
	inputBuffer := entry.NewAlloca(bufferType)

	formatStr := ctx.newConstString("builtin.prompt$"+strconv.Itoa(limit)+".format", "%"+strconv.Itoa(limit)+"s")
	formatStrPtr := constant.NewGetElementPtr(formatStr.Typ.ElemType, formatStr, zero, zero)
	bufferPtr := entry.NewGetElementPtr(bufferType, inputBuffer, constant.NewInt(types.I32, 0), constant.NewInt(types.I32, 0))
	entry.NewCall(scanf, formatStrPtr, bufferPtr)

	// i8PtrToBuffer := entry.NewBitCast(inputBuffer, types.I8Ptr)
	// i8PtrToBuffer.LocalName = "input"

	entry.NewCall(ok, bufferPtr)
	entry.NewRet(nil)

	return prompt
}
