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
var functions = make([]*function, 0) // TODO: Support files and multiple modules
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
	frm := newFrame(module, "main", ret, true, false)
	err := handleBody(frm, nodes)
	if err != nil {
		return nil, fmt.Errorf("error converting root nodes to IR: %w", err)
	}
	for i, f := range functions {
		err := f.finish()
		if err != nil {
			return nil, fmt.Errorf("[%d]: %w", i, err)
		}
	}
	return module, nil
}

type callable interface { // TODO: Won't need callable once everything is a function
	value.Value
	Node() ast.Node
	Value() value.Value
	Params(frm *function) ([]*Param, error)
	Name() string
}

type function struct { // TODO: ABC Normalize everything to be a function, this means that a function can hold own params and a LLVMString method
	c  int
	id string
	*ir.Func
	params []*Param
	mod    *ir.Module
	block  *ir.Block
	node   ast.Node
	ret    value.Value
	inner  map[string]any
	outer  map[string]any
	// we can use names in inner/outer to figure out if it's the same param being passed as the closure env
	// TODO: it doesn't matter really, just the amount matters, all params can be ptr type (even ints)
}

func (f *function) Node() ast.Node {
	return f.node
}

func (f *function) Value() value.Value {
	return f
}

func (f *function) Params(frm *function) ([]*Param, error) {
	return f.params, nil
}

// a Param could be a function (which could be a closure) or a value
type Param struct {
	*ir.Param
	node *ast.Type
}

func (p *Param) Node() ast.Node {
	return p.node
}

func (p *Param) Value() value.Value {
	return p.Param
}

func (p *Param) Params(frm *function) ([]*Param, error) {
	if p.node == nil {
		return make([]*Param, 0), nil // TODO: Might be false if params set inside the actual param
	}
	// TODO: This could also work for primitive types, by having the primitive param have a callback that returns the value
	// TODO: Inefficient
	params := make([]*Param, len(p.node.Values))
	if p.node.Value != "" {
		// TODO: We know what this is for int `((@int), @int)`, what about string?
		return nil, fmt.Errorf("value param: %q, value %v, values %+v", p.Name(), p.node.Value, p.node.Values)
	}
	for i, p := range p.node.Values {
		p, err := handleParam(frm, p)
		if err != nil {
			return nil, fmt.Errorf("handleParam: %q %w", p.node.Name, err)
		}
		params[i] = p
	}
	return params, nil
}

func handleBuiltIn(frm *function, name string, apply *ast.Apply) (any, bool) {
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

	mod := frm.mod

	// TODO: extern is not optimal
	switch name {
	case "@str":
		builtin[id] = types.I8Ptr
	case "@int":
		builtin[id] = types.I32
	case "@float":
		builtin[id] = types.Float
	case "@ieq", "@igt":
		x := &Param{Param: ir.NewParam("x", types.I32), node: &ast.Type{Name: "x", Value: "@int"}}
		y := &Param{Param: ir.NewParam("y", types.I32), node: &ast.Type{Name: "y", Value: "@int"}}
		cbtrue := &Param{Param: ir.NewParam("true", types.NewPointer(types.NewFunc(types.Void))), node: &ast.Type{Name: "true", Values: []*ast.Type{{Value: "@int"}}}}
		cbfalse := &Param{Param: ir.NewParam("false", types.NewPointer(types.NewFunc(types.Void))), node: &ast.Type{Name: "false", Values: []*ast.Type{{Value: "@int"}}}}

		f := mod.NewFunc("builtin."+strings.TrimLeft(name, "@"), types.Void, x.Param, y.Param, cbtrue.Param, cbfalse.Param)
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

		builtin[id] = &function{mod: mod, Func: f, params: []*Param{x, y, cbtrue, cbfalse}}
	case "@add":
		ok := &Param{Param: ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I32))), node: &ast.Type{Name: "ok", Values: []*ast.Type{{Value: "@int"}}}}
		x := &Param{Param: ir.NewParam("x", types.I32), node: &ast.Type{Name: "x", Value: "@int"}}
		y := &Param{Param: ir.NewParam("y", types.I32), node: &ast.Type{Name: "y", Value: "@int"}}
		f := mod.NewFunc("builtin.add", types.Void, x.Param, y.Param, ok.Param)
		entry := f.NewBlock("entry")
		res := entry.NewAdd(x, y)
		entry.NewCall(ok, res)
		entry.NewRet(nil)
		builtin[id] = &function{mod: mod, Func: f, params: []*Param{x, y, ok}}
	case "@mul":
		ok := &Param{Param: ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I32))), node: &ast.Type{Name: "ok", Values: []*ast.Type{{Value: "@int"}}}}
		x := &Param{Param: ir.NewParam("x", types.I32), node: &ast.Type{Name: "x", Value: "@int"}}
		y := &Param{Param: ir.NewParam("y", types.I32), node: &ast.Type{Name: "y", Value: "@int"}}
		f := mod.NewFunc("builtin.mul", types.Void, x.Param, y.Param, ok.Param)
		entry := f.NewBlock("entry")
		res := entry.NewMul(x, y)
		entry.NewCall(ok, res)
		entry.NewRet(nil)
		builtin[id] = &function{mod: mod, Func: f, params: []*Param{x, y, ok}}
	case "@unsafe.libc.atoi":
		in := &Param{Param: ir.NewParam("in", types.I8Ptr), node: &ast.Type{Name: "in", Value: "@str"}}
		ok := &Param{Param: ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I32))), node: &ast.Type{Name: "ok", Values: []*ast.Type{{Value: "@int"}}}}
		f := mod.NewFunc("unsafe.libc.atoi", types.Void, in.Param, ok.Param)
		entry := f.NewBlock("entry")
		atoi := mod.NewFunc("atoi", types.I32, in.Param)
		res := entry.NewCall(atoi, in.Param) // TODO: Use in.Param() ?
		// convert i32 res to i64
		// res64 := entry.NewZExt(res, types.I32)
		entry.NewCall(ok, res)
		entry.NewRet(nil)
		builtin[id] = &function{mod: mod, Func: f, params: []*Param{in, ok}}
	case "@unsafe.libc.puts":
		str := &Param{Param: ir.NewParam("str", types.I8Ptr), node: &ast.Type{Name: "str", Value: "@str"}}
		ok := &Param{Param: ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I32))), node: &ast.Type{Name: "ok", Values: []*ast.Type{{Value: "@int"}}}}
		f := mod.NewFunc("unsafe.libc.puts", types.Void, str.Param, ok.Param)
		entry := f.NewBlock("entry")
		puts := mod.NewFunc("puts", types.I32, str.Param)
		res := entry.NewCall(puts, str.Param)
		// res64 := entry.NewZExt(res, types.I32)
		entry.NewCall(ok, res)
		entry.NewRet(nil)
		builtin[id] = &function{mod: mod, Func: f, params: []*Param{str, ok}}
	case "@unsafe.libc.printf":
		fmt := &Param{Param: ir.NewParam("fmt", types.I8Ptr), node: &ast.Type{Name: "fmt", Value: "@str"}}
		f := mod.NewFunc("printf", types.I32, fmt.Param)
		f.Sig.Variadic = true
		builtin[id] = &function{mod: mod, Func: f, params: []*Param{fmt}}
	case "@unsafe.libc.scanf":
		fmt := &Param{Param: ir.NewParam("fmt", types.I8Ptr), node: &ast.Type{Name: "fmt", Value: "@str"}}
		f := mod.NewFunc("__isoc99_scanf", types.I32, fmt.Param)
		f.Sig.Variadic = true
		builtin[id] = &function{mod: mod, Func: f, params: []*Param{fmt}}
	case "@unsafe.libc.getchar":
		f := mod.NewFunc("getchar", types.I32)
		builtin[id] = &function{mod: mod, Func: f, params: []*Param{}}
	case "@unsafe.libc.fgets":
		fmt := &Param{Param: ir.NewParam("fmt", types.I8Ptr), node: &ast.Type{Name: "fmt", Value: "@str"}}
		n := &Param{Param: ir.NewParam("n", types.I32), node: &ast.Type{Name: "n", Value: "@int"}}
		stream := &Param{Param: ir.NewParam("stream", types.I8Ptr), node: &ast.Type{Name: "stream", Value: "@str"}}
		f := mod.NewFunc("fgets", types.I8Ptr, fmt.Param, n.Param, stream.Param)
		builtin[id] = &function{mod: mod, Func: f, params: []*Param{fmt, n, stream}}
	case "@printf":
		fmtStr, ok := comptimeArgs[0].(string)
		if !ok {
			panic("@printf first format argument must be a compile time string literal") // TODO: Remove panic
		}
		specs, err := libcutils.ParsePrintfFmt(fmtStr)
		if !ok {
			panic(fmt.Sprintf("error parsing printf format string %q: %s", fmtStr, err)) // TODO: Remove panic
		}
		fmtS := &Param{Param: ir.NewParam("fmt", types.I8Ptr), node: &ast.Type{Name: "fmt", Value: "@str", CompTime: true}}
		params := []*Param{fmtS}
		params2 := []*ir.Param{fmtS.Param}
		args := []value.Value{fmtS.Param}

		for i, spec := range specs {
			var t1 *ast.Type
			var t2 types.Type
			switch spec.Specifier {
			case "i", "d":
				t1 = &ast.Type{Name: "p" + strconv.Itoa(i), Value: "@int"}
				t2 = types.I32
			case "s":
				t1 = &ast.Type{Name: "p" + strconv.Itoa(i), Value: "@str"}
				t2 = types.I8Ptr
			default:
				panic(fmt.Sprintf("specifier not supported in printf: %q", spec.Specifier)) // TODO: Remove panic
			}
			p := ir.NewParam("p"+strconv.Itoa(i), t2)
			params = append(params, &Param{Param: p, node: t1})
			params2 = append(params2, p)
			args = append(args, p)
		}
		nm := strings.TrimLeft(id, "@")
		f := mod.NewFunc(nm, types.Void, params2...)

		printfAny, ok := handleBuiltIn(frm, "@unsafe.libc.printf", nil)
		if !ok {
			panic("@unsafe.libc.printf not found")
		}
		printf := printfAny.(*function).Func

		entry := f.NewBlock("entry")
		entry.NewCall(printf, args...) // TODO: In prod ignore error since it's compile time checked already, in dev panic
		entry.NewRet(nil)

		builtin[id] = &function{mod: mod, Func: f, params: params}
		return builtin[id], true
	case "@prompt":
		limit, ok := comptimeArgs[0].(int)
		if !ok {
			panic("@prompt limit must be an int") // TODO: Remove panic
		}
		f := declarePrompt(id, frm, limit)
		builtin[id] = f
		return f, true
	case "@std":
		frm.addType("string", types.I8Ptr)
		frm.addType("float", types.Float)
		frm.addType("int", types.I32)
		f := mod.NewFunc("std", types.Void)
		entry := f.NewBlock("entry")
		entry.NewRet(nil)
		builtin[id] = &function{mod: mod, Func: f}
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

func (f *function) updateParams() error {
	if f == nil {
		return fmt.Errorf("no function to update params")
	}

	f.Func.Params = make([]*ir.Param, len(f.params))
	for i, p := range f.params {
		// TODO: For some reason this breaks the `test_19_if.rgo` one
		// p, err := handleParam(f, p.node)
		// if err != nil {
		// 	return fmt.Errorf("updateParams: handleParam: %w", err)
		// }
		f.Func.Params[i] = p.Param
	}

	// closure params can use NewPtr()

	if len(f.Func.Params) != len(f.params) {
		return fmt.Errorf("len(f.Func.Params) != len(f.params): %d != %d", len(f.Func.Params), len(f.params))
	}

	// Fix func signature
	f.Typ = nil
	paramTypes := make([]types.Type, len(f.params))
	for i, p := range f.Func.Params {
		t := p.Type()
		paramTypes[i] = t
	}

	sig := types.NewFunc(f.Sig.RetType, paramTypes...)
	f.Sig = sig
	f.Type() // Compute type.
	return nil
}

func newFrame(mod *ir.Module, id string, ret value.Value, hasBody, isVariadic bool) *function {
	var retType types.Type = types.Void
	if ret != nil {
		retType = ret.Type()
	}
	fnc := mod.NewFunc(id, retType)
	fnc.Sig.Variadic = isVariadic
	var entry *ir.Block = nil
	if hasBody {
		entry = fnc.NewBlock("entry") // TODO: Rename entry to 0?
	}
	frm := &function{
		id:    id,
		mod:   mod,
		Func:  fnc,
		block: entry,
		inner: make(map[string]any),
		outer: make(map[string]any),
		ret:   ret,
	}
	functions = append(functions, frm)
	return frm
}

func (f *function) new(name string, node ast.Node, params []*Param, hasBody, isVariadic bool) *function {
	id := strings.Builder{}
	id.WriteString(f.id)
	id.WriteString(".")
	if name == "" {
		id.WriteString(strconv.Itoa(f.c))
		f.c++
	} else {
		id.WriteString(name)
	}
	frm := newFrame(f.mod, id.String(), nil, hasBody, isVariadic)
	frm.params = params
	frm.node = node // Apply or Function

	// copy outer frame inner to this frame outer
	for k, v := range f.outer {
		frm.outer[k] = v
	}
	for k, v := range f.inner {
		frm.outer[k] = v
	}

	// Copy params to inner (overwriting any outer values, shadowing)
	for _, p := range params {
		if p.Name() == "" {
			panic("param has no name")
		}
		frm.inner[p.Name()] = p
	}

	// Replace every outer value that is not constant to be a global
	for k, v := range frm.outer {
		// if already part of inner, skip
		if _, ok := frm.inner[k]; ok {
			continue
		}
		switch n := v.(type) {
		case *function:
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
		case *ir.InstLoad:
			if _, ok := n.Src.(*ir.Global); !ok {
				panic(fmt.Sprintf("outer load type not supported: %s %T", k, n.Src))
			}
			var v value.Value = n
			isInt := n.Type().Equal(types.I32)
			if isInt {
				v = f.block.NewAlloca(n.Type()) // TODO: maybe there's a better way
				f.block.NewStore(n.Src, v)
			}
			load := frm.block.NewLoad(v.Type(), n.Src)
			if isInt {
				load = frm.block.NewLoad(n.Type(), load) // TODO: maybe there's a better way
			}
			load.LocalName = k
			frm.outer[k] = load
		case *Param:
			// This is a hack around the llvm limitation of not being able to use the stack outside of the function
			// TODO: Recycle these globals based on max amount of needed globals instead of creating on demand
			g := f.mod.NewGlobalDef("", constant.NewNull(types.NewPointer(nil))) // TODO: Create a pull request to merge opaque pointer support into llir/llvm
			var v value.Value = n
			isInt := n.Type().Equal(types.I32)
			if isInt {
				v = f.block.NewAlloca(n.Type()) // TODO: maybe there's a better way
				f.block.NewStore(n, v)
			}
			f.block.NewStore(v, g)
			load := frm.block.NewLoad(v.Type(), g)
			if isInt {
				load = frm.block.NewLoad(n.Type(), load) // TODO: maybe there's a better way
			}
			load.LocalName = k
			frm.outer[k] = load
		default:
			panic(fmt.Sprintf("outer type not supported: %q with type %T, called from %q", k, v, f.id))
		}
	}
	return frm
}

func (f *function) addType(name string, typ types.Type) error {
	if name == "" {
		return fmt.Errorf("type name cannot be empty")
	}
	if _, ok := f.inner[name]; ok {
		return fmt.Errorf("add type: label already exists: %v", name)
	}
	f.inner[name] = typ
	return nil
}

func (f *function) addValue(name string, fn value.Value) error {
	if name == "" {
		return fmt.Errorf("adding value for function %q: value name cannot be empty", f.id)
	}
	if _, ok := f.inner[name]; ok {
		return fmt.Errorf("add value: label already exists: %v", name)
	}
	f.inner[name] = fn
	return nil
}

func (f *function) getValue(name string) (value.Value, error) {
	v, ok := f.inner[name]
	if !ok {
		v, ok = f.outer[name]
		if !ok {
			return nil, fmt.Errorf("value %q not found in inner or outer scope", name)
		}
	}
	if v, ok := v.(value.Value); ok {
		return v, nil
	}
	return nil, fmt.Errorf("reference %q found, but is not a value", name)
}

func (f *function) getType(name string) (types.Type, error) {
	v, ok := f.inner[name]
	if !ok {
		v, ok = f.outer[name]
		if !ok {
			v, ok = handleBuiltIn(f, name, nil)
			if !ok {
				return nil, fmt.Errorf("type %q not found in inner or outer scope", name)
			}
		}
	}
	if v, ok := v.(types.Type); ok {
		return v, nil
	}
	return nil, fmt.Errorf("reference %q found, but is not a type", name)
}

func (f *function) finish() error { // TODO: ABC No need for finish if it's rendered using LLVMString
	err := f.updateParams()
	if err != nil {
		return fmt.Errorf("error updating function %q params: %w", f.Name(), err)
	}
	if f.block != nil {
		f.block.NewRet(f.ret)
	}
	return nil
}

func handleParam(frm *function, t *ast.Type) (*Param, error) {
	if t == nil {
		return nil, fmt.Errorf("param node == nil")
	}

	// Handle basic types
	if t.Value != "" {
		typ, err := frm.getType(t.Value)
		if err != nil {
			return nil, fmt.Errorf("failed to get type: %w", err)
		}
		return &Param{Param: ir.NewParam(t.Name, typ), node: t}, nil
	}

	// Handle function pointer
	innerTypes := make([]types.Type, 0)
	for _, t := range t.Values {
		p, err := handleParam(frm, t)
		if err != nil {
			return nil, fmt.Errorf("failed to handle function pointer: %w", err)
		}
		innerTypes = append(innerTypes, p.Typ)
	}
	return &Param{
		Param: ir.NewParam(t.Name, types.NewPointer(types.NewFunc(types.Void, innerTypes...))),
		node:  t,
	}, nil
}

func handleNode(frm *function, node ast.Node, param *Param) (value.Value, error) {
	var err error
	var callee callable
	// Remember it internally
	switch n := node.(type) { // TODO: This would have been the Declaration AST node
	case *ast.Function:
		callee, err = handleFunctionNode(frm, n)
		if err != nil {
			return nil, fmt.Errorf("error handling function: %w", err)
		}
	case *ast.Apply:
		callee, err = handleApplyNode(frm, n)
		if err != nil {
			return nil, fmt.Errorf("error handling apply: %w", err)
		}
	case *ast.Label:
		// TODO: Allow variadic builtins here
		v, err := frm.getValue(n.Of) // TODO: this is used identically in two places
		if err != nil {
			return nil, fmt.Errorf("error finding label: %w", err)
		}
		return v, nil
	case *ast.IntLiteral:
		// TODO: slightly duplicated
		v := constant.NewInt(types.I32, int64(n.Value)) // TODO: Assuming 64-bit integers
		return v, nil
	case *ast.StringLiteral:
		// TODO: slightly duplicated
		globalStr := frm.newConstString(n.Name, n.Value)
		v := constant.NewGetElementPtr(globalStr.Typ.ElemType, globalStr, zero, zero)
		return v, nil
	default:
		return nil, fmt.Errorf("unsupported declaration type: %T", node)
	}

	if param != nil {
		// The apply is adding more params than the callback param supports
		callbackParams, err := param.Params(frm)
		if err != nil {
			return nil, fmt.Errorf("error getting callback params: %w", err)
		}
		params, err := callee.Params(frm)
		if err != nil {
			return nil, fmt.Errorf("error getting apply params: %w", err)
		}
		if len(callbackParams) < len(params) {
			return nil, fmt.Errorf("adding more arguments than the callback %q expects, %q expected %d arguments, got %d", callee.Name(), param.Name(), len(callbackParams), len(params))
		}
	}
	return callee, nil
}

func handleFunctionNode(frm *function, n *ast.Function) (*function, error) {
	name := n.Name
	if name == "" {
		name = strconv.Itoa(frm.c)
		frm.c++
	}

	// handle params
	params := make([]*Param, len(n.Params))
	for i, p := range n.Params {
		p, err := handleParam(frm, p)
		if err != nil {
			return nil, fmt.Errorf("[%d]: func: %w", i, err)
		}
		params[i] = p
	}

	fn := frm.new(name, n, params, true, false)

	err := handleBody(fn, n.Body)
	if err != nil {
		return nil, fmt.Errorf("error handling function body for function %q: %w", n.Name, err)
	}
	return fn, nil
}

func handleApplyNode(frm *function, n *ast.Apply) (callable, error) {
	var callee callable
	switch f := n.Callee.(type) {
	case *ast.Function:
		fn, err := handleFunctionNode(frm, f)
		if err != nil {
			return nil, fmt.Errorf("error handling anon function %q: %w", n.Name, err)
		}
		callee = fn
	case *ast.Label:
		var err error
		v, ok := handleBuiltIn(frm, f.Of, n) // TODO: handle this in handleCallable
		if !ok {
			v, err = frm.getValue(f.Of)
			if err != nil {
				return nil, fmt.Errorf("error finding label: %w", err)
			}
		}
		if callee, ok = v.(callable); !ok {
			return nil, fmt.Errorf("reference found, but is not callable: %q", f.Of)
		}
	default:
		// TODO: handle other types of apply
		return nil, fmt.Errorf("unsupported callee type: %T", n.Callee)
	}

	calleeParams, err := callee.Params(frm)
	if err != nil {
		return nil, fmt.Errorf("error getting callee params: %w", err)
	}

	// The apply is adding more params than the callee supports
	if len(calleeParams) < len(n.Arguments) {
		return nil, fmt.Errorf("%q expected params %d, got %d", callee.Name(), len(calleeParams), len(n.Arguments))
	}

	args := make([]value.Value, 0, len(n.Arguments))
	for i, arg := range n.Arguments {
		v, err := handleNode(frm, arg, calleeParams[i])
		if err != nil {
			return nil, fmt.Errorf("[%d] error converting argument: %w", i, err)
		}
		args = append(args, v)
	}

	params := make([]*Param, len(calleeParams[len(args):]))
	for i, p := range calleeParams[len(args):] {
		params[i] = p
		args = append(args, p)
	}
	apply := frm.new(n.Name, n, params, true, false) // call frame

	// Call the callee from inside the apply helper
	apply.block.NewCall(callee, args...) // TODO: This might not be enough args
	return apply, nil
}

func handleBody(frm *function, nodes []ast.Node) error {
	for i, node := range nodes {
		// TODO: Now that Label() is available, could simply use handleNode (for strings could do the pointer outside of handleNode? or get the thing the value points at?)
		// A declaration can either a function, a constant or an alias to a function, constant or parameter.
		// A declaration can never be a variable.
		switch n := node.(type) { // TODO: This would have been the Declaration AST node
		case *ast.Function:
			callee, err := handleFunctionNode(frm, n)
			if err != nil {
				return fmt.Errorf("[%d] in body: error handling function %q: %w", i, n.Name, err)
			}
			if n.Name == "" {
				frm.block.NewCall(callee)
			} else {
				err = frm.addValue(n.Name, callee)
				if err != nil {
					return fmt.Errorf("add fn ref: %w", err)
				}
			}
		case *ast.Apply:
			if n.Name == "" {
				var ok bool
				var callee callable
				if label, ok := n.Callee.(*ast.Label); ok {
					// TODO: could return a function, since everything is a function
					if builtin, ok := handleBuiltIn(frm, label.Of, n); ok {
						if callee, ok = builtin.(callable); !ok {
							return fmt.Errorf("handle builtin call: %v", label.Of)
						}
					}
				}
				if callee == nil {
					// TODO: Change this to support any type
					v, err := handleNode(frm, n.Callee, nil)
					if err != nil {
						return fmt.Errorf("handle call: %w", err)
					}
					if callee, ok = v.(callable); !ok {
						return fmt.Errorf("expected callable but got %T", v)
					}
				}

				// Just call the unsafe functions directly, hence they are unsafe
				args := make([]value.Value, 0, len(n.Arguments))
				for i, arg := range n.Arguments {
					v, err := handleNode(frm, arg, nil)
					if err != nil {
						return fmt.Errorf("[%d] error converting argument: %w", i, err)
					}
					args = append(args, v)
				}
				frm.block.NewCall(callee, args...)
			} else {
				callee, err := handleApplyNode(frm, n)
				if err != nil {
					return fmt.Errorf("[%d] in body: error handling apply: %w", i, err)
				}
				err = frm.addValue(n.Name, callee)
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
			err := frm.addValue(n.Name, v)                  // TODO: check for empty
			if err != nil {
				return fmt.Errorf("add int ref: %w", err)
			}
		case *ast.StringLiteral:
			// TODO: Duplicate code in handleNode
			if n.Name == "" {
				return fmt.Errorf("anonymous string literal statement is not supported")
			}
			globalStr := frm.newConstString(n.Name, n.Value)
			err := frm.addValue(n.Name, globalStr) // TODO: check for empty
			if err != nil {
				return fmt.Errorf("add str ref: %w", err)
			}
		default:
			return fmt.Errorf("unsupported declaration type: %T", node)
		}
	}
	return nil
}

func (frm *function) newConstString(name string, value string) *ir.Global {
	mod := frm.mod
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

func declarePrompt(id string, frm *function, limit int) *function { // TODO: Could just use fscan with the limit inside prompt
	name := "builtin." + strings.TrimLeft(id, "@")
	// TODO: Limit should be a compile time param

	scanfAny, isFunc := handleBuiltIn(frm, "@unsafe.libc.scanf", nil) // TODO: handle this in handleCallable
	if !isFunc {
		panic("@unsafe.libc.scanf not found")
	}

	scanf, isCallable := scanfAny.(callable)
	if !isCallable {
		panic("@unsafe.libc.scanf not a function")
	}

	// TODO: Maybe this can be implemented in rgo?
	// TODO: Maybe use scanf("%3s", string)?
	// limitParam := ir.NewParam("limit", types.I32) // TODO: Make it dynamic and safe? Compile time value?
	limitParam := &Param{Param: ir.NewParam("limit", types.NewPointer(types.NewFunc(types.Void, types.I8Ptr))), node: &ast.Type{Name: "limit", Value: "@int", CompTime: true}}
	ok := &Param{Param: ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I8Ptr))), node: &ast.Type{Name: "ok", Values: []*ast.Type{{Value: "@str"}}}}
	prompt := frm.mod.NewFunc(name, types.Void, limitParam.Param, ok.Param)
	entry := prompt.NewBlock("entry")

	// Allocate a buffer with size equal to limit + 1 (for null terminator).
	bufferType := types.NewArray(uint64(limit+1), types.I8)
	inputBuffer := entry.NewAlloca(bufferType)

	formatStr := frm.newConstString("builtin.prompt$"+strconv.Itoa(limit)+".format", "%"+strconv.Itoa(limit)+"s")
	formatStrPtr := constant.NewGetElementPtr(formatStr.Typ.ElemType, formatStr, zero, zero)
	bufferPtr := entry.NewGetElementPtr(bufferType, inputBuffer, constant.NewInt(types.I32, 0), constant.NewInt(types.I32, 0))
	entry.NewCall(scanf, formatStrPtr, bufferPtr)

	// i8PtrToBuffer := entry.NewBitCast(inputBuffer, types.I8Ptr)
	// i8PtrToBuffer.LocalName = "input"

	entry.NewCall(ok, bufferPtr)
	entry.NewRet(nil)

	return &function{
		mod:  frm.mod,
		Func: prompt, node: &ast.Function{
			Name:   "prompt",
			Params: []*ast.Type{limitParam.node, ok.node},
		},
		params: []*Param{limitParam, ok},
	}
}
