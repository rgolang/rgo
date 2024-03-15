package lex

import (
	"fmt"
	"io"
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
	i.ByteOffset -= len([]byte(t.Value))
	i.LineRuneOffset -= utf8.RuneCountInString(t.Value)
	i.LineOffset -= len([]byte(t.Value))
	return &i
}

func NewToken(r *reader.Reader, typ TokenType, value string) *Token {
	return &Token{info: r.Info, Type: typ, Value: value}
}

var hasDebug bool

type Scanner struct {
	Index  uint
	Token  *Token
	Reader *reader.Reader
}

func New(r io.ReadSeeker) *Scanner {
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
		return NewToken(r, TokenEnd, "")
	}
	if lastChar == '\n' || lastChar == '\r' {
		for unicode.IsSpace(lastChar) || lastChar == '\n' || lastChar == '\r' {
			lastChar = r.ReadRune()
			print("read: %s - end", lastChar)
		}
		if lastChar == reader.EOF {
			return NewToken(r, TokenEnd, "")
		}
		r.UnreadRune()
		print("unrd: %s", lastChar)
		return NewToken(r, TokenNewline, string(lastChar))
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
				return NewToken(r, TokenDocComment, strings.TrimSpace("///"+docComment.String()))
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
		return NewToken(r, TokenIdentifier, idStr.String())
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
		return NewToken(r, TokenInt, numStr.String())
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
		return NewToken(r, TokenString, str.String())
	}

	print("read: %s - symbol", lastChar)
	switch lastChar {
	case '@':
		return NewToken(r, TokenAt, string(lastChar))
	case '(':
		return NewToken(r, TokenLeftParen, string(lastChar))
	case ')':
		return NewToken(r, TokenRightParen, string(lastChar))
	case '{':
		return NewToken(r, TokenLeftBrace, string(lastChar))
	case '}':
		return NewToken(r, TokenRightBrace, string(lastChar))
	case ',':
		return NewToken(r, TokenComma, string(lastChar))
	case ':':
		return NewToken(r, TokenColon, string(lastChar))
	case ';':
		return NewToken(r, TokenSemicolon, string(lastChar))
	case '.':
		return NewToken(r, TokenDot, string(lastChar))
	case '!':
		return NewToken(r, TokenExclaim, string(lastChar))
	case '?':
		return NewToken(r, TokenQuestion, string(lastChar))
	case '-':
		return NewToken(r, TokenBinOp, string(lastChar))
	case '+':
		return NewToken(r, TokenBinOp, string(lastChar))
	case '*':
		return NewToken(r, TokenBinOp, string(lastChar))
	case '<':
		return NewToken(r, TokenBinOp, string(lastChar))
	case '"':
		return NewToken(r, TokenDoubleQuote, string(lastChar))
	case '\'':
		return NewToken(r, TokenSingleQuote, string(lastChar))
	}

	return NewToken(r, TokenUnknown, string(lastChar))
}
