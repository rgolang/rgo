package lex

import (
	"bufio"
	"strings"
	"testing"

	"github.com/rgolang/rgo/reader"
	"github.com/stretchr/testify/require"
)

func newReader(input string) *reader.Reader {
	return reader.New(bufio.NewReader(strings.NewReader(input)))
}

func TestNextTokenSimple(t *testing.T) {
	hasDebug = true

	// run multiple tests from a tt loop
	for _, tc := range []struct {
		name       string
		input      string
		file       string
		line       int
		lineoffset int
		byteoffset int
	}{
		{
			name:       "no trailing whitespace",
			line:       0,
			lineoffset: 0,
			byteoffset: 0,
			input:      `1234`,
		},
		{
			name:       "with surrounding whitespace",
			line:       2,
			lineoffset: 3,
			byteoffset: 5,
			input: `

			1234

			`,
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			r := newReader(tc.input)
			tok := firstToken(r)
			require.Equal(t, TokenInt, tok.Type)
			require.Equal(t, "1234", tok.Value)
			require.Equal(t, "", tok.Info().File) // TODO: add file support
			require.Equal(t, tc.line, tok.Info().Line)
			require.Equal(t, tc.lineoffset, tok.Info().LineOffset)
			require.Equal(t, tc.byteoffset, tok.Info().ByteOffset)
			tok = nextToken(r)
			require.Equal(t, TokenEnd, tok.Type)
		})
	}
}

func TestStringTokenSimple(t *testing.T) {
	hasDebug = true

	// run multiple tests from a tt loop
	for _, tc := range []struct {
		name       string
		input      string
		file       string
		line       int
		lineoffset int
		byteoffset int
	}{
		{
			name:       "no trailing whitespace",
			line:       0,
			lineoffset: 0,
			byteoffset: 0,
			input:      `"1234"`,
		},
		{
			name:       "with surrounding whitespace",
			line:       2,
			lineoffset: 3,
			byteoffset: 5,
			input: `

			"1234"

			`,
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			r := newReader(tc.input)
			tok := firstToken(r)
			require.Equal(t, TokenString, tok.Type)
			require.Equal(t, `"1234"`, tok.Value)
			require.Equal(t, "", tok.Info().File) // TODO: add file support
			require.Equal(t, tc.line, tok.Info().Line)
			require.Equal(t, tc.lineoffset, tok.Info().LineOffset)
			require.Equal(t, tc.byteoffset, tok.Info().ByteOffset)
			tok = nextToken(r)
			require.Equal(t, TokenEnd, tok.Type)
		})
	}
}

func TestTokensWithDot(t *testing.T) {
	hasDebug = true

	r := newReader("x.1.2")
	tok := firstToken(r)
	require.Equal(t, TokenIdentifier, tok.Type)
	require.Equal(t, `x`, tok.Value)
	require.Equal(t, "", tok.Info().File) // TODO: add file support
	require.Equal(t, 0, tok.Info().Line)
	require.Equal(t, 0, tok.Info().LineOffset)
	require.Equal(t, 0, tok.Info().ByteOffset)

	tok = nextToken(r)
	require.Equal(t, TokenDot, tok.Type)
	require.Equal(t, `.`, tok.Value)
	require.Equal(t, "", tok.Info().File) // TODO: add file support
	require.Equal(t, 0, tok.Info().Line)
	require.Equal(t, 1, tok.Info().LineOffset)
	require.Equal(t, 1, tok.Info().ByteOffset)

	tok = nextToken(r)
	require.Equal(t, TokenInt, tok.Type)
	require.Equal(t, `1`, tok.Value)
	require.Equal(t, "", tok.Info().File) // TODO: add file support
	require.Equal(t, 0, tok.Info().Line)
	require.Equal(t, 2, tok.Info().LineOffset)
	require.Equal(t, 2, tok.Info().ByteOffset)

	tok = nextToken(r)
	require.Equal(t, TokenDot, tok.Type)
	require.Equal(t, `.`, tok.Value)
	require.Equal(t, "", tok.Info().File) // TODO: add file support
	require.Equal(t, 0, tok.Info().Line)
	require.Equal(t, 3, tok.Info().LineOffset)
	require.Equal(t, 3, tok.Info().ByteOffset)

	tok = nextToken(r)
	require.Equal(t, TokenInt, tok.Type)
	require.Equal(t, `2`, tok.Value)
	require.Equal(t, "", tok.Info().File) // TODO: add file support
	require.Equal(t, 0, tok.Info().Line)
	require.Equal(t, 4, tok.Info().LineOffset)
	require.Equal(t, 4, tok.Info().ByteOffset)

	tok = nextToken(r)
	require.Equal(t, TokenEnd, tok.Type)
}

// func TestTokenFloat(t *testing.T) {
// 	hasDebug = true

// 	r := newReader("1.2")
// 	tok := firstToken(r)
// 	require.Equal(t, TokenFloat, tok.Type)
// 	require.Equal(t, `1.2`, tok.Value)
// 	require.Equal(t, "", tok.Info().File) // TODO: add file support
// 	require.Equal(t, 0, tok.Info().Line)
// 	require.Equal(t, 0, tok.Info().LineOffset)
// 	require.Equal(t, 0, tok.Info().ByteOffset)

// 	tok = nextToken(r)
// 	require.Equal(t, TokenEnd, tok.Type)
// }

func TestSimpleExpression(t *testing.T) {
	hasDebug = true
	r := newReader("1+2")
	tok := firstToken(r)
	require.Equal(t, TokenInt, tok.Type)
	require.Equal(t, "1", tok.Value)
	require.Equal(t, "", tok.Info().File) // TODO: add file support
	require.Equal(t, 0, tok.Info().Line)
	require.Equal(t, 0, tok.Info().LineOffset)
	require.Equal(t, 0, tok.Info().ByteOffset)

	tok = nextToken(r)
	require.Equal(t, TokenBinOp, tok.Type)
	require.Equal(t, "+", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenInt, tok.Type)
	require.Equal(t, "2", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenEnd, tok.Type)
}

func TestNextToken(t *testing.T) {
	hasDebug = true

	input := `
	/// my doc comment.  
	my_functiön {} // This is a comment
	x + y
	12.3; 123
	my_definition: 1234
	`
	r := newReader(input)
	var tok *Token

	tok = firstToken(r)
	require.Equal(t, TokenDocComment, tok.Type)
	require.Equal(t, "/// my doc comment.", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenIdentifier, tok.Type)
	require.Equal(t, "my_functiön", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenLeftBrace, tok.Type)

	tok = nextToken(r)
	require.Equal(t, TokenRightBrace, tok.Type)

	tok = nextToken(r)
	require.Equal(t, TokenNewline, tok.Type)

	tok = nextToken(r)
	require.Equal(t, TokenIdentifier, tok.Type)
	require.Equal(t, "x", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenBinOp, tok.Type)
	require.Equal(t, "+", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenIdentifier, tok.Type)
	require.Equal(t, "y", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenNewline, tok.Type)

	tok = nextToken(r)
	require.Equal(t, TokenInt, tok.Type)
	require.Equal(t, "12", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenDot, tok.Type)
	require.Equal(t, ".", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenInt, tok.Type)
	require.Equal(t, "3", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenSemicolon, tok.Type)

	tok = nextToken(r)
	require.Equal(t, TokenInt, tok.Type)
	require.Equal(t, "123", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenNewline, tok.Type)

	tok = nextToken(r)
	require.Equal(t, TokenIdentifier, tok.Type)
	require.Equal(t, "my_definition", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenColon, tok.Type)
	require.Equal(t, ":", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenInt, tok.Type)
	require.Equal(t, "1234", tok.Value)

	tok = nextToken(r)
	require.Equal(t, TokenEnd, tok.Type)
}

func TestStringEscape(t *testing.T) {
	hasDebug = true
	r := newReader(`"hi\n"`)
	tok := firstToken(r)
	require.Equal(t, TokenString, tok.Type)

	// Using Go's raw string literal to accurately represent the expected lexer output
	expected := "\"hi\n\""
	require.Equal(t, expected, tok.Value)
}
