// This example produces LLVM IR generating "Hello, World" output.

package main

import (
	"bufio"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/rgolang/rgo/ast"
	"github.com/rgolang/rgo/lex"
	"github.com/rgolang/rgo/llvm"
)

func dfr(cb func() error) {
	if err := cb(); err != nil {
		panic(err)
	}
}

func main() {
	path := os.Args[1]

	// strip extension from filename (any extension)
	ext := filepath.Ext(path)
	name := strings.TrimSuffix(path, ext)

	// Read hello.rgo
	rf, err := os.OpenFile(path, os.O_RDONLY, 0644)
	if err != nil {
		log.Fatalf("open src file: %v", err)
	}
	defer dfr(rf.Close)

	// Generate the code
	reader := bufio.NewReader(rf)
	lexer := lex.New(reader)
	parser := ast.New(lexer)

	tree, err := parser.Parse()
	if err != nil {
		log.Fatalf("parse: %v", err)
	}
	code, err := llvm.ToIR(tree)
	if err != nil {
		log.Fatalf("llvm: %v", err)
	}

	// Open hello.ll for writing
	wf, err := os.OpenFile(name+".ll", os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0644)
	if err != nil {
		log.Fatalf("open dst file: %v", err)
	}
	defer dfr(wf.Close)

	// Write code to hello.ll
	if _, err := code.WriteTo(wf); err != nil {
		log.Fatalf("write: %v", err)
	}
}
