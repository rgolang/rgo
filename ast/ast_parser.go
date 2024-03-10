package ast

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/rgolang/rgo/lex"
	"github.com/rgolang/rgo/reader"
)

// TODO: Add a callback that accepts a func or a static global and adds it to the llvm module for JIT compilation.
type Parser struct {
	lex *lex.Scanner
}

type EOF error

// TODO: Get rid of Node, everything is a function, even ints and strings
type Node interface { // TODO: use later to add a method to convert to llvm IR
	Info() *reader.Info
	Label() string // TODO: using Label because Name can't be used
}

type IntLiteral struct {
	*lex.Token
	Name  string
	Value int
}

func (n *IntLiteral) Label() string {
	return n.Name
}

type FloatLiteral struct {
	*lex.Token
	Name  string
	Value float64
}

func (n *FloatLiteral) Label() string {
	return n.Name
}

type StringLiteral struct {
	*lex.Token
	Name  string
	Value string
}

func (n *StringLiteral) Label() string {
	return n.Name
}

type Type struct {
	*lex.Token
	Name     string
	Values   []*Type
	Value    string
	CompTime bool
}

func (n *Type) Label() string {
	return n.Name
}

type Function struct { // TODO: make these private?
	*lex.Token
	Name   string
	Params []*Type
	Body   []Node
}

func (n *Function) Label() string {
	return n.Name
}

type Apply struct {
	*lex.Token
	Name      string
	Callee    Node
	Arguments []Node
}

func (n *Apply) Label() string {
	return n.Name
}

type Label struct {
	*lex.Token
	Name      string
	Of        string
	IsBuiltIn bool
}

func (n *Label) Label() string {
	return n.Name
}

func New(scanner *lex.Scanner) *Parser {
	return &Parser{
		lex: scanner,
	}
}

func (p *Parser) Parse() ([]Node, error) { // TODO: Should return the next block of statements that are ready to be inserted into the AST
	expr, err := p.handleBody()
	if err != nil {
		return nil, fmt.Errorf("failed to parse top level: %w", err)
	}
	return expr, nil
}

func (p *Parser) handleBody() ([]Node, error) {
	statements := []Node{}
	c := 0
	for {
		switch p.lex.Token.Type {
		case lex.TokenEnd:
			return statements, nil
		case lex.TokenNewline, lex.TokenComma:
			// continue to the next token
			p.lex.NextToken() // eat the newline or `,`
		case lex.TokenLeftBrace, lex.TokenLeftParen:
			// Handle anon function
			var stmt Node
			fn, err := p.handleFunction()
			if err != nil {
				return nil, fmt.Errorf("[%v] failed to handle anonymous function declaration: %w", c, err)
			}
			stmt = fn

			switch p.lex.Token.Type {
			case lex.TokenLeftParen:
				args, err := p.handleArgs()
				if err != nil {
					return nil, fmt.Errorf("[%d] failed to parse anonymous function call arguments: %w", c, err)
				}
				stmt = &Apply{
					Token:     p.lex.Token,
					Callee:    fn,
					Arguments: args,
				}
				statements = append(statements, stmt)
			case lex.TokenNewline, lex.TokenComma, lex.TokenEnd:
				statements = append(statements, stmt)
			default:
				return nil, fmt.Errorf("[%v] expected anonymous function to be called or to terminate, got %v", c, p.lex.Token.Info())
			}
		case lex.TokenAt, lex.TokenIdentifier:
			// Assume it's a label, could be a named or unnamed one
			label, err := p.handleLabel()
			if err != nil {
				return nil, fmt.Errorf("[%v] failed to handle label: %w", c, err)
			}
			if p.lex.Token.Type == lex.TokenColon {
				if label.IsBuiltIn {
					return nil, fmt.Errorf("declaring built-in labels is not supported")
				}

				p.lex.NextToken()                  // eat the colon
				expr, err := p.handleDeclaration() // Handle reference, literal or applied function declarations
				if err != nil {
					return nil, fmt.Errorf("failed to parse declaration: %w", err)
				}

				// TODO: this can be improved
				switch e := expr.(type) {
				case *Apply:
					e.Name = label.Of
				case *Function:
					e.Name = label.Of
				case *Label:
					e.Name = label.Of
				case *Type:
					e.Name = label.Of
				case *IntLiteral:
					e.Name = label.Of
				case *FloatLiteral:
					e.Name = label.Of
				case *StringLiteral:
					e.Name = label.Of
				default:
					return nil, fmt.Errorf("failed to parse declaration result: %T", expr)
				}

				statements = append(statements, expr)
				break
			}
			stmt, err := p.handleCall(label)
			if err != nil {
				return nil, fmt.Errorf("[%v] failed to handle call statement: %w", c, err)
			}
			statements = append(statements, stmt)
		case lex.TokenRightBrace: // TODO: find a better way to handle this
			return statements, nil
		default:
			return nil, fmt.Errorf("[%v] failed to handle statement, unknown token: %q: %+v", c, p.lex.Token.Value, p.lex.Token.Info())
		}
		c++
	}
}

func (p *Parser) handleNameWithDots(callee string) (string, error) {
	b := strings.Builder{}
	b.WriteString(callee) // this allows us to pass an @ prefix
	// Handling names with dots
	for p.lex.Token.Type == lex.TokenDot {
		p.lex.NextToken() // eat the dot
		if p.lex.Token.Type != lex.TokenIdentifier {
			return "", fmt.Errorf("expected identifier after dot, got %q: %+v", p.lex.Token.Value, p.lex.Token.Info())
		}
		b.WriteRune('.')
		b.WriteString(p.lex.Token.Value)
		p.lex.NextToken() // eat the next part of the name
	}
	return b.String(), nil
}

func (p *Parser) handleCall(calleeLabel *Label) (Node, error) {
	calleeName, err := p.handleNameWithDots(calleeLabel.Of)
	if err != nil {
		return nil, fmt.Errorf("failed to handle function call name: %w", err)
	}
	calleeLabel.Of = calleeName
	expr, err := p.handleApply(calleeLabel)
	if err != nil {
		return nil, fmt.Errorf("failed to handle function call of %q: %w", calleeLabel.Of, err)
	}
	return expr, nil
}

func (p *Parser) handleDeclaration() (Node, error) {
	var expr Node
	switch p.lex.Token.Type {
	case lex.TokenLeftParen, lex.TokenLeftBrace: // TODO: handle function declaration without parameters as `{}`
		fn, err := p.handleFunction()
		if err != nil {
			return nil, fmt.Errorf("failed to parse function declaration: %w", err)
		}
		expr = fn
	case lex.TokenInt, lex.TokenString:
		lit, err := p.handleLiteral()
		if err != nil {
			return nil, fmt.Errorf("failed to parse literal: %w", err)
		}
		expr = lit
	case lex.TokenAt, lex.TokenIdentifier:
		label, err := p.handleLabel()
		if err != nil {
			return nil, fmt.Errorf("failed to parse identifier: %w", err)
		}
		expr = label

		// Not all function expressions contain `()`, but when they don't, it can stay as a reference
		if p.lex.Token.Type == lex.TokenLeftParen {
			// This must be an applied function
			expr, err = p.handleApply(label)
			if err != nil {
				return nil, fmt.Errorf("failed to handle function application: %w", err)
			}
		}
	default:
		return nil, fmt.Errorf("unknown token %q in definition: %+v", p.lex.Token.Value, p.lex.Token.Info())
	}

	// A direct call of the function
	if p.lex.Token.Type == lex.TokenLeftParen {
		if fn, ok := expr.(*Function); ok {
			args, err := p.handleArgs()
			if err != nil {
				return nil, fmt.Errorf("failed to parse lambda function declaration arguments: %w", err)
			}
			expr = &Apply{
				Token:     p.lex.Token,
				Callee:    fn,
				Arguments: args,
			}
		}
	}
	return expr, nil
}

func (p *Parser) handleLabel() (*Label, error) {
	isBuiltIn := false
	refName := p.lex.Token.Value
	if p.lex.Token.Type == lex.TokenAt {
		isBuiltIn = true
		p.lex.NextToken() // eat the @
		if p.lex.Token.Type != lex.TokenIdentifier {
			return nil, fmt.Errorf("expected call identifier, got %q: %+v", p.lex.Token.Value, p.lex.Token.Info())
		}
		refName = "@" + p.lex.Token.Value
	}

	// Handle an name (e.g., a variable or a string reference)
	p.lex.NextToken() // Move past the identifier

	// TODO: aren't all references just functions?
	return &Label{ // Assuming you have a Reference type for identifiers
		Token:     p.lex.Token,
		Of:        refName,
		IsBuiltIn: isBuiltIn,
	}, nil
}

func (p *Parser) handleApply(calleeLabel *Label) (*Apply, error) { // TODO: Move ToIR to Statement
	args, err := p.handleArgs()
	if err != nil {
		return nil, fmt.Errorf("failed to parse arguments: %w", err)
	}
	return &Apply{
		Token:     p.lex.Token,
		Callee:    calleeLabel,
		Arguments: args,
	}, nil
}

func (p *Parser) handleArgs() ([]Node, error) {
	switch p.lex.Token.Type {
	// Allow fully qualified functions to be called without parenthesis
	case lex.TokenEnd, lex.TokenNewline, lex.TokenComma:
		return []Node{}, nil
	case lex.TokenLeftParen:
		p.lex.NextToken() // Eat `(`
		var args []Node

		// Handle case where there are no args inside parenthesis
		if p.lex.Token.Type == lex.TokenRightParen {
			p.lex.NextToken() // Eat `)`
			return args, nil
		}

		// Parse arguments until closing parenthesis
		for p.lex.Token.Type != lex.TokenRightParen {
			expr, err := p.handleDeclaration() // anonymous id, generate on the spot
			if err != nil {
				return nil, fmt.Errorf("failed to handle argument: %w", err)
			}
			args = append(args, expr)

			// Check for comma or closing parenthesis
			if p.lex.Token.Type == lex.TokenComma {
				p.lex.NextToken() // Eat `,`
			} else if p.lex.Token.Type != lex.TokenRightParen {
				return nil, fmt.Errorf("expected ',' or ')', found %q: %+v", p.lex.Token.Value, p.lex.Token.Info())
			}
		}

		p.lex.NextToken() // Eat `)`
		return args, nil
	default:
		return nil, fmt.Errorf("unexpected token %q for args: %+v", p.lex.Token.Value, p.lex.Token.Info())
	}
}

func (p *Parser) handleLiteral() (x Node, err error) {
	tok := p.lex.Token
	switch tok.Type {
	case lex.TokenInt:
		x, err := strconv.Atoi(tok.Value)
		if err != nil {
			return nil, fmt.Errorf("failed to parse int %q at %+v", tok.Info(), tok.Value)
		}
		p.lex.NextToken()
		return &IntLiteral{Token: p.lex.Token, Value: x}, nil
	case lex.TokenString:
		p.lex.NextToken()
		return &StringLiteral{Token: p.lex.Token, Value: tok.Value[1 : len(tok.Value)-1]}, nil
	default:
		return x, fmt.Errorf("unknown token when expecting a literal: %#v", p.lex.Token)
	}
}

func (p *Parser) handleFunction() (*Function, error) {
	fn := Function{
		Token: p.lex.Token,
	}

	var err error
	if p.lex.Token.Type == lex.TokenLeftParen { // TODO: this loop is to prevent `MyFunc: MyType{}` func declarations, which could be confusing, or not?
		x, err := p.handleType("")
		if err != nil {
			return nil, fmt.Errorf("failed to parse function type: %w", err)
		}
		fn.Params = x.Values
	}

	// Expecting '{' for function body
	if p.lex.Token.Type != lex.TokenLeftBrace {
		return nil, fmt.Errorf("expected '{', found %q: %+v", p.lex.Token.Value, p.lex.Token.Info())
	}
	p.lex.NextToken() // eat '{'

	// Parse function body
	fn.Body, err = p.handleBody()
	if err != nil {
		return nil, fmt.Errorf("failed to parse function body: %w", err)
	}

	// Expecting '}' after function body
	if p.lex.Token.Type != lex.TokenRightBrace {
		return nil, fmt.Errorf("expected '}', found %q", p.lex.Token.Value)
	}
	p.lex.NextToken() // eat '}'

	return &fn, nil
}

func (p *Parser) handleType(name string) (*Type, error) {
	prefix := ""
	if p.lex.Token.Type == lex.TokenAt {
		prefix = "@"
		p.lex.NextToken() // Eat `@`
	}

	switch p.lex.Token.Type {
	case lex.TokenRightParen: // Empty type `()`
		p.lex.NextToken() // eat the )
		return &Type{
			Token: p.lex.Token,
			Name:  name,
		}, nil
	case lex.TokenIdentifier:
		// TODO: handle references
		// Handle basic types
		typeOrName := p.lex.Token.Value
		p.lex.NextToken() // Move past the type identifier
		if p.lex.Token.Type == lex.TokenColon {
			p.lex.NextToken() // eat ':', this might be followed by an identifier or `@` or `(`
			typ, err := p.handleType(typeOrName)
			if err != nil {
				return nil, fmt.Errorf("failed to parse named type: %w", err)
			}
			return typ, nil
		}
		var compTime bool
		if p.lex.Token.Type == lex.TokenExclaim {
			p.lex.NextToken() // eat '!'
			compTime = true
		}
		return &Type{
			Token:    p.lex.Token,
			Name:     name,
			Value:    prefix + typeOrName,
			CompTime: compTime,
		}, nil
	case lex.TokenLeftParen:
		p.lex.NextToken() // eat '('

		var types []*Type
		for p.lex.Token.Type != lex.TokenRightParen {
			if p.lex.Token.Type == lex.TokenEnd {
				return nil, fmt.Errorf("unexpected end of input while parsing function type: %+v", p.lex.Token.Info())
			}

			singleType, err := p.handleType("") // anonymous type
			if err != nil {
				return nil, fmt.Errorf("failed to parse function type in type: %w", err)
			}
			types = append(types, singleType)

			// Handle ',' between parameters or end of parameters
			if p.lex.Token.Type == lex.TokenComma || p.lex.Token.Type == lex.TokenNewline {
				p.lex.NextToken() // eat `,` or `\n`
			}
		}
		p.lex.NextToken() // eat ')'

		return &Type{
			Token:  p.lex.Token,
			Name:   name,
			Values: types,
		}, nil
	default:
		return nil, fmt.Errorf("unexpected token %q while parsing type: %+v", p.lex.Token.Value, p.lex.Token.Info())
	}
}
