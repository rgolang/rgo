package lex

import (
	"bufio"
	"fmt"
	"strings"
	"unicode"
	"unicode/utf8"

	"github.com/rgolang/rgo/reader"
)

type TokenType int

// Define types iota
const (
	TokenStart TokenType = iota // Start with 0
	TokenEnd
	TokenAt
	TokenIdentifier
	TokenInt
	TokenString
	TokenUnknown
	TokenLeftParen
	TokenRightParen
	TokenLeftBrace
	TokenRightBrace
	TokenComma
	TokenDot
	TokenBinOp // TODO: Not yet used
	TokenNewline
	TokenColon
	TokenSemicolon
	TokenExclaim     // TODO: Not yet used
	TokenQuestion    // TODO: Not yet used
	TokenDoubleQuote // TODO: Isn't this a string?
	TokenSingleQuote // TODO: Isn't this a string?
	TokenDocComment
	TokenBackslash
	TokenTab
)

type Token struct {
	Type  TokenType
	info  reader.Info
	Value string
}

func (t Token) Info() *reader.Info {
	i := t.info
	i.ByteOffset -= len(t.Value)
	i.LineOffset -= utf8.RuneCountInString(t.Value)
	return &i
}

var hasDebug bool

type Scanner struct {
	Index  uint
	Token  *Token
	Reader *reader.Reader
}

func New(r *bufio.Reader) *Scanner {
	rdr := reader.New(r)
	return &Scanner{
		Index:  0,
		Token:  firstToken(rdr),
		Reader: rdr,
	}
}

func (s *Scanner) NextToken() *Token {
	token := s.Token
	s.Token = nextToken(s.Reader)
	return token
}

func print(s string, args ...any) {
	if hasDebug {
		props := make([]any, len(args))
		for i, a := range args {
			switch v := a.(type) {
			case rune:
				switch v {
				case '\r':
					props[i] = "\\r"
				case '\n':
					props[i] = "\\n"
				case '\t':
					props[i] = "\\t"
				case ' ':
					props[i] = "\\s"
				case reader.EOF:
					props[i] = "\\EOF"
				default:
					props[i] = string(v)
				}
			default:
				props[i] = a
			}
		}
		fmt.Printf(s+"\n", props...)
	}
}

func firstToken(r *reader.Reader) *Token {
	lastChar := r.ReadRune()
	print("read: initial char %s", lastChar)

	// Consume any leading whitespace, including newlines.
	for unicode.IsSpace(lastChar) {
		print("read: initial whitespace")
		lastChar = r.ReadRune()
	}
	r.UnreadRune()
	print("unrd: initial char %s", lastChar)
	return nextToken(r)
}

func nextToken(r *reader.Reader) *Token {
	var lastChar rune = ' '
	print("prep: next token initial fake space")

	// Skip any spaces.
	for unicode.IsSpace(lastChar) && lastChar != '\n' && lastChar != '\r' {
		lastChar = r.ReadRune()
		print("read: %s - space", lastChar)
	}

	// Consume end of statement.
	if lastChar == reader.EOF {
		return &Token{info: r.Info, Type: TokenEnd}
	}
	if lastChar == '\n' || lastChar == '\r' {
		for unicode.IsSpace(lastChar) || lastChar == '\n' || lastChar == '\r' {
			lastChar = r.ReadRune()
			print("read: %s - end", lastChar)
		}
		if lastChar == reader.EOF {
			return &Token{info: r.Info, Type: TokenEnd}
		}
		r.UnreadRune()
		print("unrd: %s", lastChar)
		return &Token{info: r.Info, Type: TokenNewline}
	}

	// Consume comments until end of line.
	if lastChar == '/' {
		lastChar = r.ReadRune()
		print("read: %s - slash", lastChar)
		if lastChar == '/' {
			// Check if doc comment.
			lastChar = r.ReadRune()
			print("read: %s - comment", lastChar)
			if lastChar == '/' {
				// Skip any slashes.
				for lastChar == '/' {
					print("read: %s - redundant slash", lastChar)
					lastChar = r.ReadRune()
				}
				var docComment strings.Builder
				for lastChar != '\n' && lastChar != '\r' && lastChar != reader.EOF {
					docComment.WriteRune(lastChar)
					lastChar = r.ReadRune()
					print("read: %s - doc", lastChar)
				}
				for lastChar == '\n' || lastChar == '\r' {
					lastChar = r.ReadRune()
					print("read: %s - redundant newline", lastChar)
				}
				r.UnreadRune()
				print("unrd: %s", lastChar)
				return &Token{info: r.Info, Type: TokenDocComment, Value: strings.TrimSpace("///" + docComment.String())}
			}
			for lastChar != '\n' && lastChar != '\r' && lastChar != reader.EOF {
				lastChar = r.ReadRune()
				print("read: %s - comment", lastChar)
			}
			r.UnreadRune()
			print("unrd: %s", lastChar)
			return nextToken(r)
		}
	}

	// Consume identifiers.
	if unicode.IsLetter(lastChar) {
		var idStr strings.Builder
		for unicode.IsLetter(lastChar) || unicode.IsDigit(lastChar) || lastChar == '_' {
			idStr.WriteRune(lastChar)
			lastChar = r.ReadRune()
			print("read: %s - identifier", lastChar)
		}
		r.UnreadRune()
		print("unrd: %s", lastChar)
		return &Token{info: r.Info, Type: TokenIdentifier, Value: idStr.String()}
	}

	// Consume number.
	if unicode.IsDigit(lastChar) {
		var numStr strings.Builder
		for unicode.IsDigit(lastChar) {
			numStr.WriteRune(lastChar)
			lastChar = r.ReadRune()
			print("read: %s - number", lastChar)
		}
		r.UnreadRune()
		print("unrd: %s", lastChar)
		return &Token{info: r.Info, Type: TokenInt, Value: numStr.String()}
	}

	// Consume strings.
	if lastChar == '"' || lastChar == '\'' {
		var str strings.Builder
		str.WriteRune(lastChar)

		quoteType := lastChar
		lastChar = r.ReadRune() // eat the quote
		print("read: %s - string", lastChar)
		str.WriteRune(lastChar)

		for lastChar != quoteType {
			if lastChar == reader.EOF {
				panic("missing closing quote") // TODO: don't panic here
			}
			lastChar = r.ReadRune()
			print("read: %s - string", lastChar)

			// Handle escape sequences (TODO: This might mess up the line offset, need to separate textual representation from the value)
			if lastChar == '\\' {
				lastChar = r.ReadRune() // consume `\`
				print("read: %s - string escape", lastChar)
				switch lastChar {
				case 'n':
					str.WriteRune('\n')
				case 'r':
					str.WriteRune('\r')
				default:
					str.WriteRune('\\')     // Keep as-is
					str.WriteRune(lastChar) // Unrecognized escape, keep as-is
				}
				continue
			} else {
				// Non-escape, write it
				str.WriteRune(lastChar)
			}
		}
		return &Token{info: r.Info, Type: TokenString, Value: str.String()}
	}

	print("read: %s - symbol", lastChar)
	switch lastChar {
	case '@':
		return &Token{info: r.Info, Type: TokenAt, Value: string(lastChar)}
	case '(':
		return &Token{info: r.Info, Type: TokenLeftParen, Value: string(lastChar)}
	case ')':
		return &Token{info: r.Info, Type: TokenRightParen, Value: string(lastChar)}
	case '{':
		return &Token{info: r.Info, Type: TokenLeftBrace, Value: string(lastChar)}
	case '}':
		return &Token{info: r.Info, Type: TokenRightBrace, Value: string(lastChar)}
	case ',':
		return &Token{info: r.Info, Type: TokenComma, Value: string(lastChar)}
	case ':':
		return &Token{info: r.Info, Type: TokenColon, Value: string(lastChar)}
	case ';':
		return &Token{info: r.Info, Type: TokenSemicolon, Value: string(lastChar)}
	case '.':
		return &Token{info: r.Info, Type: TokenDot, Value: string(lastChar)}
	case '!':
		return &Token{info: r.Info, Type: TokenExclaim, Value: string(lastChar)}
	case '?':
		return &Token{info: r.Info, Type: TokenQuestion, Value: string(lastChar)}
	case '-':
		return &Token{info: r.Info, Type: TokenBinOp, Value: string(lastChar)}
	case '+':
		return &Token{info: r.Info, Type: TokenBinOp, Value: string(lastChar)}
	case '*':
		return &Token{info: r.Info, Type: TokenBinOp, Value: string(lastChar)}
	case '<':
		return &Token{info: r.Info, Type: TokenBinOp, Value: string(lastChar)}
	case '"':
		return &Token{info: r.Info, Type: TokenDoubleQuote, Value: string(lastChar)}
	case '\'':
		return &Token{info: r.Info, Type: TokenSingleQuote, Value: string(lastChar)}
	}

	return &Token{info: r.Info, Type: TokenUnknown, Value: string(lastChar)}
}
