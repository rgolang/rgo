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
}

type IntLiteral struct {
	*lex.Token
	Name  string
	Value int
}

type FloatLiteral struct {
	*lex.Token
	Name  string
	Value float64
}

type StringLiteral struct {
	*lex.Token
	Name  string
	Value string
}

type Type struct {
	*lex.Token
	Name     string
	Values   []*Type
	Value    string
	CompTime bool
}

type Signature struct {
}

type Function struct { // TODO: make these private?
	*lex.Token
	Name   string
	Params []*Type
	Body   []Node
}

type Apply struct {
	*lex.Token
	Name      string
	Of        string
	Function  *Function
	Arguments []Node
}

type Alias struct {
	*lex.Token
	Name string
	Of   string
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
		case lex.TokenAt:
			// Must be a function call
			p.lex.NextToken() // eat the @
			if p.lex.Token.Type != lex.TokenIdentifier {
				return nil, fmt.Errorf("[%v] expected call identifier, got %v", c, p.lex.Token.Info())
			}
			callee := p.lex.Token.Value
			p.lex.NextToken() // eat the call identifier
			stmt, err := p.handleCall("@" + callee)
			if err != nil {
				return nil, fmt.Errorf("[%v] failed to handle @call statement: %w", c, err)
			}
			statements = append(statements, stmt)
		case lex.TokenLeftBrace, lex.TokenLeftParen:
			// Handle anon function
			var stmt Node
			fn, err := p.handleFunction("")
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
					Function:  fn,
					Arguments: args,
				}
				statements = append(statements, stmt)
			case lex.TokenNewline, lex.TokenComma, lex.TokenEnd:
				statements = append(statements, stmt)
			default:
				return nil, fmt.Errorf("[%v] expected anonymous function to be called or to terminate, got %v", c, p.lex.Token.Info())
			}
		case lex.TokenIdentifier:
			calleeOrName := p.lex.Token.Value
			p.lex.NextToken() // eat the declaration name
			if p.lex.Token.Type == lex.TokenColon {
				p.lex.NextToken()                              // eat the colon
				expr, err := p.handleDeclaration(calleeOrName) // Handle reference, literal or applied function declarations
				if err != nil {
					return nil, fmt.Errorf("failed to parse declaration: %w", err)
				}
				// A direct call of the function
				if p.lex.Token.Type == lex.TokenLeftParen {
					if fn, ok := expr.(*Function); ok {
						args, err := p.handleArgs()
						if err != nil {
							return nil, fmt.Errorf("failed to parse lambda function declaration arguments: %w", err)
						}
						fn.Name = ""
						expr = &Apply{
							Name:      calleeOrName,
							Token:     p.lex.Token,
							Function:  fn,
							Arguments: args,
						}
					}
				}
				statements = append(statements, expr)
				break
			}
			stmt, err := p.handleCall(calleeOrName)
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

func (p *Parser) handleCall(callee string) (Node, error) {
	callee, err := p.handleNameWithDots(callee)
	if err != nil {
		return nil, fmt.Errorf("failed to handle function call name: %w", err)
	}
	expr, err := p.handleApply("", callee)
	if err != nil {
		return nil, fmt.Errorf("failed to handle function call of %q: %w", callee, err)
	}
	return expr, nil
}

func (p *Parser) handleDeclaration(name string) (Node, error) {
	switch p.lex.Token.Type {
	case lex.TokenLeftParen, lex.TokenLeftBrace: // TODO: handle function declaration without parameters as `{}`
		fn, err := p.handleFunction(name)
		if err != nil {
			return nil, fmt.Errorf("failed to parse function declaration: %w", err)
		}
		return fn, nil
	case lex.TokenInt, lex.TokenString:
		expr, err := p.handleLiteral(name)
		if err != nil {
			return nil, fmt.Errorf("failed to parse literal: %w", err)
		}
		return expr, nil
	case lex.TokenIdentifier:
		expr, err := p.handleIdentifier(name)
		if err != nil {
			return nil, fmt.Errorf("failed to parse identifier: %w", err)
		}
		return expr, nil
	case lex.TokenAt:
		expr, err := p.handleIdentifier(name)
		if err != nil {
			return nil, fmt.Errorf("failed to parse identifier: %w", err)
		}
		return expr, nil
	default:
		return nil, fmt.Errorf("unknown token %q in definition: %+v", p.lex.Token.Value, p.lex.Token.Info())
	}
}

func (p *Parser) handleIdentifier(name string) (Node, error) {
	refName := p.lex.Token.Value
	if p.lex.Token.Type == lex.TokenAt {
		p.lex.NextToken() // eat the @
		if p.lex.Token.Type != lex.TokenIdentifier {
			return nil, fmt.Errorf("expected call identifier, got %q: %+v", p.lex.Token.Value, p.lex.Token.Info())
		}
		refName = "@" + p.lex.Token.Value
	}

	// Handle an name (e.g., a variable or a string reference)
	p.lex.NextToken() // Move past the identifier

	// Not all function expressions contain `()`, but when they don't, it can stay as a reference
	if p.lex.Token.Type == lex.TokenLeftParen {
		// This must be an applied function
		expr, err := p.handleApply(name, refName)
		if err != nil {
			return nil, fmt.Errorf("failed to handle function application: %w", err)
		}
		return expr, nil
	}

	// TODO: aren't all references just functions?
	return &Alias{ // Assuming you have a Reference type for identifiers
		Token: p.lex.Token,
		Name:  name,
		Of:    refName,
	}, nil
}

func (p *Parser) handleApply(name string, callee string) (*Apply, error) { // TODO: Move ToIR to Statement
	args, err := p.handleArgs()
	if err != nil {
		return nil, fmt.Errorf("failed to parse arguments: %w", err)
	}
	return &Apply{
		Token:     p.lex.Token,
		Name:      name,
		Of:        callee,
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
			expr, err := p.handleDeclaration("") // anonymous id, generate on the spot
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

func (p *Parser) handleLiteral(name string) (x Node, err error) {
	tok := p.lex.Token
	switch tok.Type {
	case lex.TokenInt:
		x, err := strconv.Atoi(tok.Value)
		if err != nil {
			return nil, fmt.Errorf("failed to parse int %q at %+v", tok.Info(), tok.Value)
		}
		p.lex.NextToken()
		return &IntLiteral{Name: name, Token: p.lex.Token, Value: x}, nil
	case lex.TokenString:
		p.lex.NextToken()
		return &StringLiteral{Name: name, Token: p.lex.Token, Value: tok.Value[1 : len(tok.Value)-1]}, nil
	default:
		return x, fmt.Errorf("unknown token when expecting a literal: %#v", p.lex.Token)
	}
}

func (p *Parser) handleFunction(name string) (*Function, error) {
	fn := Function{
		Token: p.lex.Token,
		Name:  name,
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
