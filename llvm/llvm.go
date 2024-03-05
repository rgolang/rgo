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

type callable interface {
	value.Value
	Node() ast.Node
	Value() value.Value
	Params(frm *function) ([]*Param, error)
	IsVariadic() bool
	Name() string
}

type function struct {
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

func (f *function) IsVariadic() bool {
	return f.Func.Sig.Variadic
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

func (p *Param) IsVariadic() bool {
	return false // Can't tell, so defaulting to false for now // TODO:
}

func handleBuiltIn(frm *function, name string, apply *ast.Apply) (any, bool) {
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
	// if _, ok := builtin[name]; ok {
	// 	return builtin[name], ok
	// }

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

func (f *function) addFunc(name string, fn callable) error {
	if name == "" {
		return fmt.Errorf("function name cannot be empty")
	}
	if _, ok := f.inner[name]; ok {
		return fmt.Errorf("add function: label already exists: %v", name)
	}
	f.inner[name] = fn
	return nil
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

func (f *function) getCallable(name string, applyNode *ast.Apply) (callable, error) {
	v, ok := f.inner[name]
	if !ok {
		v, ok = f.outer[name]
		if !ok {
			v, ok = handleBuiltIn(f, name, applyNode)
			if !ok {
				return nil, fmt.Errorf("function %q not found in inner or outer scope", name)
			}
		}
	}
	if v, ok := v.(callable); ok {
		return v, nil
	}
	return nil, fmt.Errorf("reference %q found, but is not callable", name)
}

func (f *function) finish() error {
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

func handleVariadicArg(frm *function, node ast.Node) (string, value.Value, error) { // TODO: IMprove this
	// Remember it internally
	switch n := node.(type) { // TODO: This would have been the Declaration AST node
	case *ast.Alias:
		v, err := frm.getValue(n.Of)
		if err != nil {
			return n.Name, nil, fmt.Errorf("variadic arg: %w", err)
		}
		return n.Name, v, nil
	case *ast.IntLiteral:
		// TODO: slightly duplicated
		v := constant.NewInt(types.I32, int64(n.Value)) // TODO: Assuming 64-bit integers
		return n.Name, v, nil
	case *ast.StringLiteral:
		// TODO: slightly duplicated
		globalStr := newConstString(n.Name, n.Value)
		v := constant.NewGetElementPtr(globalStr.Typ.ElemType, globalStr, zero, zero) // TODO: Could this re-use the above?
		frm.mod.Globals = append(frm.mod.Globals, globalStr)
		return n.Name, v, nil
	default:
		return "", nil, fmt.Errorf("unsupported declaration type: %T", node)
	}
}

func handleArgument(frm *function, node ast.Node, param *Param) (string, value.Value, error) {
	// Remember it internally
	switch n := node.(type) { // TODO: This would have been the Declaration AST node
	case *ast.Function:
		// The apply is adding more params than the callback param supports
		callbackParams, err := param.Params(frm)
		if err != nil {
			return n.Name, nil, fmt.Errorf("fn, error getting callback params: %w", err)
		}

		if len(callbackParams) < len(n.Params) {
			return n.Name, nil, fmt.Errorf("%q function has more parameters than the callback %q expects, expected %d, got %d", frm.id, param.Name(), len(callbackParams), len(n.Params))
		}

		fn, err := handleFunctionNode(frm, n)
		if err != nil {
			return n.Name, nil, fmt.Errorf("error handling function: %w", err)
		}
		return n.Name, fn, nil
	case *ast.Apply:
		// TODO: Check if we're calling a callback

		// TODO: Check how many args are being expected by the callee and how many are provided by the caller (to detect a closure)
		// TODO: Check how many args are being expected by the callee and how many are provided by the caller (to detect a closure)
		callee, err := frm.getCallable(n.Of, n)
		if err != nil {
			return n.Name, nil, fmt.Errorf("error finding callee: %w", err)
		}

		calleeParams, err := callee.Params(frm)
		if err != nil {
			return n.Name, nil, fmt.Errorf("error getting callee params: %w", err)
		}

		// The apply is adding more params than the callee supports
		if len(calleeParams) < len(n.Arguments) {
			return n.Name, nil, fmt.Errorf("apply is adding more arguments than the callee expects, expected %d arguments, got %d", len(calleeParams), len(n.Arguments))
		}

		// The apply is adding more params than the callback param supports
		callbackParams, err := param.Params(frm)
		if err != nil {
			return n.Name, nil, fmt.Errorf("apply, error getting callback params: %w", err)
		}

		args := make([]value.Value, 0, len(n.Arguments))
		for i, arg := range n.Arguments {
			_, v, err := handleArgument(frm, arg, calleeParams[i])
			if err != nil {
				return n.Name, nil, fmt.Errorf("[%d] error converting argument: %w", i, err)
			}
			args = append(args, v)
		}

		// it's basically a new function (which could be a closure)
		params := make([]*Param, len(calleeParams[len(args):]))
		for i, p := range calleeParams[len(args):] {
			params[i] = p
			args = append(args, p)
		}
		apply := frm.new(n.Name, n, params, true, false) // call frame

		if len(callbackParams) < len(apply.params) {
			return n.Name, nil, fmt.Errorf("apply is adding more arguments than the callback %q expects, %q expected %d arguments, got %d", apply.Name(), param.Name(), len(callbackParams), len(apply.params))
		}

		// Call the callee from inside the apply helper
		apply.block.NewCall(callee, args...) // TODO: This might not be enough args

		return n.Name, apply, nil
	case *ast.Alias:
		v, err := frm.getValue(n.Of)
		if err != nil {
			return n.Name, nil, fmt.Errorf("error finding reference: %w", err)
		}
		return n.Name, v, nil
	case *ast.IntLiteral:
		// TODO: slightly duplicated
		v := constant.NewInt(types.I32, int64(n.Value)) // TODO: Assuming 64-bit integers
		return n.Name, v, nil
	case *ast.StringLiteral:
		// TODO: slightly duplicated
		globalStr := newConstString(n.Name, n.Value)
		v := constant.NewGetElementPtr(globalStr.Typ.ElemType, globalStr, zero, zero)
		frm.mod.Globals = append(frm.mod.Globals, globalStr)
		return n.Name, v, nil
	default:
		return "", nil, fmt.Errorf("unsupported declaration type: %T", node)
	}
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

func handleApplyNode(frm *function, n *ast.Apply) (callable, []value.Value, error) {
	var callee callable
	if n.Of == "" && n.Function != nil {
		fn, err := handleFunctionNode(frm, n.Function)
		if err != nil {
			return nil, nil, fmt.Errorf("error handling anon function %q: %w", n.Name, err)
		}
		callee = fn
	} else {
		// TODO: Check how many args are being expected by the callee and how many are provided by the caller (to detect a closure)
		fn, err := frm.getCallable(n.Of, n)
		if err != nil {
			return nil, nil, fmt.Errorf("error finding callee: %w", err)
		}
		callee = fn
	}

	if callee.IsVariadic() {
		// Just call the unsafe functions directly, hence they are unsafe
		if n.Name != "" {
			return nil, nil, fmt.Errorf("named variadic functions not yet supported")
		}
		args := make([]value.Value, 0, len(n.Arguments))
		for i, arg := range n.Arguments {
			_, v, err := handleVariadicArg(frm, arg)
			if err != nil {
				return nil, nil, fmt.Errorf("[%d] error converting argument: %w", i, err)
			}
			args = append(args, v)
		}
		return callee, args, nil
	}
	calleeParams, err := callee.Params(frm)
	if err != nil {
		return nil, nil, fmt.Errorf("error getting callee params: %w", err)
	}

	// The apply is adding more params than the callee supports
	if len(calleeParams) < len(n.Arguments) {
		return nil, nil, fmt.Errorf("%q expected params %d, got %d", callee.Name(), len(calleeParams), len(n.Arguments))
	}

	args := make([]value.Value, 0, len(n.Arguments))
	for i, arg := range n.Arguments {
		_, v, err := handleArgument(frm, arg, calleeParams[i])
		if err != nil {
			return nil, nil, fmt.Errorf("[%d] error converting argument: %w", i, err)
		}
		args = append(args, v)
	}

	if n.Name == "" {
		// It's a direct call
		return callee, args, nil
	}

	// if it's named, then it's basically a new function (which could be a closure)
	params := make([]*Param, len(calleeParams[len(args):]))
	for i, p := range calleeParams[len(args):] {
		params[i] = p
		args = append(args, p)
	}
	apply := frm.new(n.Name, n, params, true, false) // call frame

	// Call the callee from inside the apply helper
	apply.block.NewCall(callee, args...) // TODO: This might not be enough args
	return apply, args, nil
}

func handleBody(frm *function, nodes []ast.Node) error {
	for i, node := range nodes {
		// A declaration can either a function, a constant or an alias to a function, constant or parameter.
		// A declaration can never be a variable.
		switch n := node.(type) { // TODO: This would have been the Declaration AST node
		case *ast.Function:
			fn, err := handleFunctionNode(frm, n)
			if err != nil {
				return fmt.Errorf("[%d] in body: error handling function %q: %w", i, n.Name, err)
			}
			if n.Name == "" {
				frm.block.NewCall(fn)
			} else {
				err = frm.addFunc(n.Name, fn)
				if err != nil {
					return fmt.Errorf("add fn ref: %w", err)
				}
			}
		case *ast.Apply:
			callee, args, err := handleApplyNode(frm, n) // TODO: handleApplyNode knows if it's a call, it could return a list of instructions
			if err != nil {
				return fmt.Errorf("[%d] in body: error handling apply: %w", i, err)
			}
			if n.Name == "" {
				frm.block.NewCall(callee, args...)
			} else {
				err = frm.addFunc(n.Name, callee)
				if err != nil {
					return fmt.Errorf("add fn ref: %w", err)
				}
			}
		case *ast.Alias:
			if n.Name == "" {
				callee, err := frm.getCallable(n.Of, nil)
				if err != nil {
					return fmt.Errorf("error finding callee: %w", err)
				}
				frm.block.NewCall(callee)
				return nil
			}
			v, err := frm.getValue(n.Of)
			if err != nil {
				return fmt.Errorf("error finding reference: %w", err)
			}
			err = frm.addValue(n.Name, v)
			if err != nil {
				return fmt.Errorf("add ref ref: %w", err)
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
			globalStr := newConstString(n.Name, n.Value)
			frm.mod.Globals = append(frm.mod.Globals, globalStr)
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

func newConstString(name string, value string) *ir.Global {
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

	return constStr
}

func declarePrompt(id string, frm *function, limit int) *function { // TODO: Could just use fscan with the limit inside prompt
	name := "builtin." + strings.TrimLeft(id, "@")
	// TODO: Limit should be a compile time param

	scanf, err := frm.getCallable("@unsafe.libc.scanf", nil)
	if err != nil {
		panic(err)
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

	formatStr := newConstString("builtin.prompt$"+strconv.Itoa(limit)+".format", "%"+strconv.Itoa(limit)+"s")
	frm.mod.Globals = append(frm.mod.Globals, formatStr)
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

// func declarePrompt(frm *function, limit int) *ir.Func { // TODO: Could just use fscan with the limit inside prompt
// 	// TODO: Limit should be a compile time param

// 	// TODO: Maybe this can be implemented in rgo?
// 	// TODO: Maybe use scanf("%3s", string)?
// 	ok := ir.NewParam("ok", types.NewPointer(types.NewFunc(types.Void, types.I8Ptr)))
// 	prompt := frm.mod.NewFunc("prompt", types.Void, ok)
// 	entry := prompt.NewBlock("entry")

// 	// Allocate a buffer with size equal to limit + 1 (for null terminator).
// 	bufferType := types.NewArray(uint64(limit+1), types.I8)
// 	buffer := entry.NewAlloca(bufferType)

// 	// Declare loop variables
// 	i := entry.NewAlloca(types.I32)
// 	entry.NewStore(constant.NewInt(types.I32, 0), i)

// 	// Loop condition block
// 	condBlock := prompt.NewBlock("cond")
// 	entry.NewBr(condBlock)

// 	// Loop body block
// 	loopBlock := prompt.NewBlock("loop")

// 	storeBlock := prompt.NewBlock("store")

// 	// Loop end block
// 	endBlock := prompt.NewBlock("end")

// 	// Condition check: i < limit && char != EOF && char != '\n'
// 	condBlock.NewCondBr(
// 		condBlock.NewICmp(enum.IPredULT, condBlock.NewLoad(types.I32, i), constant.NewInt(types.I32, int64(limit))),
// 		loopBlock,
// 		endBlock,
// 	)

// 	getchar, err := frm.getCallable("@unsafe.libc.getchar")
// 	if err != nil {
// 		panic(err)
// 	}

// 	// Read a character
// 	char := loopBlock.NewCall(getchar)
// 	isEOF := loopBlock.NewICmp(enum.IPredEQ, char, constant.NewInt(types.I32, -1))       // Check for EOF
// 	isNewline := loopBlock.NewICmp(enum.IPredEQ, char, constant.NewInt(types.I32, '\n')) // Check for newline
// 	loopBlock.NewCondBr(loopBlock.NewOr(isEOF, isNewline), endBlock, storeBlock)

// 	// Store the character in buffer
// 	charPtr := storeBlock.NewGetElementPtr(bufferType, buffer, constant.NewInt(types.I32, 0), storeBlock.NewLoad(types.I32, i))
// 	// Cast the character to i8 before storing
// 	castChar := storeBlock.NewTrunc(char, types.I8)
// 	storeBlock.NewStore(castChar, charPtr)

// 	// Increment index i
// 	nextI := storeBlock.NewAdd(storeBlock.NewLoad(types.I32, i), constant.NewInt(types.I32, 1))
// 	storeBlock.NewStore(nextI, i)

// 	// Loop back to condition check
// 	storeBlock.NewBr(condBlock)

// 	// Null-terminate the string in the buffer
// 	endBlock.NewStore(constant.NewInt(types.I8, 0), endBlock.NewGetElementPtr(bufferType, buffer, constant.NewInt(types.I32, 0), constant.NewInt(types.I32, int64(limit))))

// 	// Convert the buffer to a pointer to its first element (i8*)
// 	i8PtrToBuffer := endBlock.NewBitCast(buffer, types.I8Ptr)
// 	i8PtrToBuffer.LocalName = "input"

// 	// Replace the dummy placeholder arg
// 	// endCall.Args[1] = i8PtrToBuffer

// 	// if cb, ok := applyPromptIRN.Args[1].(*ir.Func); ok {
// 	// endBlock.NewCall(ir.NewFunc("whaterver", types.Void), i8PtrToBuffer)
// 	// }

// 	endBlock.NewCall(ok, i8PtrToBuffer)

// 	// endBlock.NewCall(prompt.Params[1], i8PtrToBuffer)

// 	// TODO:
// 	// Call the callback function with the buffer
// 	// switch call := callbackIRN.irv.(type) {
// 	// case *ir.Func:
// 	// 	endBlock.NewCall(call, i8PtrToBuffer)
// 	// case *ir.InstCall:
// 	// 	endBlock.Insts = append(endBlock.Insts, call)
// 	// 	ctx.block.Insts = append(ctx.block.Insts, ir.NewCall(prompt, i8PtrToBuffer))
// 	// default:
// 	// 	panic("unsupported callback type") // TODO: don't panic
// 	// }

// 	// Return from the function
// 	endBlock.NewRet(nil)

// 	return prompt
// }
