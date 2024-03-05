package reader

import (
	"bufio"
	"fmt"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"
)

func TestReader_ReadRuneRN(t *testing.T) {
	r := New(bufio.NewReader(strings.NewReader("Hëllo\r\nWörld")))
	req := require.New(t)

	lastRune := r.ReadRune() // Read 'H'
	req.Equal('H', lastRune, fmt.Sprintf("expected 'H', got: '%v'", string(lastRune)))
	req.Equal(1, r.Info.ByteOffset)
	req.Equal(1, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	lastRune = r.ReadRune() // Read 'ë'
	req.Equal('ë', lastRune, fmt.Sprintf("expected 'ë', got: '%v'", string(lastRune)))
	req.Equal(3, r.Info.ByteOffset) // 'ë' is 2 bytes in UTF-8
	req.Equal(2, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	r.ReadRune()            // Read 'l'
	r.ReadRune()            // Read 'l'
	lastRune = r.ReadRune() // Read 'o'
	req.Equal('o', lastRune, fmt.Sprintf("expected 'o', got: '%v'", string(lastRune)))

	lastRune = r.ReadRune() // Read '\r'
	req.Equal('\r', lastRune, fmt.Sprintf("expected '\\r', got: '%v'", string(lastRune)))
	req.Equal(7, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	lastRune = r.ReadRune() // Read '\n'
	req.Equal('\n', lastRune, fmt.Sprintf("expected '\\n', got: '%v'", string(lastRune)))
	req.Equal(8, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	lastRune = r.ReadRune() // Read 'W`
	req.Equal('W', lastRune, fmt.Sprintf("expected 'W', got: '%v'", string(lastRune)))
	req.Equal(9, r.Info.ByteOffset)
	req.Equal(1, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)
}

func TestReader_ReadRuneN(t *testing.T) {
	r := New(bufio.NewReader(strings.NewReader("Hëllo\nWörld")))
	req := require.New(t)

	r.ReadRune() // Read 'H'
	req.Equal(1, r.Info.ByteOffset)
	req.Equal(1, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	r.ReadRune()                    // Read 'ë'
	req.Equal(3, r.Info.ByteOffset) // 'ë' is 2 bytes in UTF-8
	req.Equal(2, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	r.ReadRune() // Read 'l'
	r.ReadRune() // Read 'l'
	r.ReadRune() // Read 'o'

	r.ReadRune() // Read '\n'
	req.Equal(7, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	r.ReadRune() // Read 'W`
	req.Equal(8, r.Info.ByteOffset)
	req.Equal(1, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)
}

func TestReader_ReadRuneR(t *testing.T) {
	r := New(bufio.NewReader(strings.NewReader("Hëllo\rWörld")))
	req := require.New(t)

	r.ReadRune() // Read 'H'
	req.Equal(1, r.Info.ByteOffset)
	req.Equal(1, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	r.ReadRune()                    // Read 'ë'
	req.Equal(3, r.Info.ByteOffset) // 'ë' is 2 bytes in UTF-8
	req.Equal(2, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	r.ReadRune() // Read 'l'
	r.ReadRune() // Read 'l'
	r.ReadRune() // Read 'o'

	r.ReadRune() // Read '\r'
	req.Equal(7, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	r.ReadRune() // Read 'W`
	req.Equal(8, r.Info.ByteOffset)
	req.Equal(1, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)
}

func TestReader_ReadRune_EOF(t *testing.T) {
	r := New(bufio.NewReader(strings.NewReader("Hë")))
	req := require.New(t)

	r.ReadRune()             // Read 'H'
	r.ReadRune()             // Read 'ë', is 2 bytes in UTF-8
	lastRune := r.ReadRune() // Attempt to read beyond EOF

	req.Equal(EOF, lastRune)
	req.Equal(4, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)
}

func TestReader_UneadRune_EOF(t *testing.T) {
	r := New(bufio.NewReader(strings.NewReader("Hë")))
	req := require.New(t)

	r.ReadRune()             // Read 'H'
	lastRune := r.ReadRune() // Read 'ë'
	req.Equal('ë', lastRune, fmt.Sprintf("expected 'ë', got: '%v'", string(lastRune)))
	req.Equal(3, r.Info.ByteOffset)
	req.Equal(2, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	lastRune = r.ReadRune() // Attempt to read beyond EOF
	req.Equal(EOF, lastRune, fmt.Sprintf("expected 'EOF', got: '%v'", string(lastRune)))
	req.Equal(4, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	r.UnreadRune()
	req.Equal(3, r.Info.ByteOffset)
	req.Equal(2, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)
}

func TestReader_UnreadRuneN(t *testing.T) {
	r := New(bufio.NewReader(strings.NewReader("Hëllo\nWörld")))
	req := require.New(t)

	// Read the first line
	lastRune := r.ReadRune() // Read 'H'
	req.Equal('H', lastRune, fmt.Sprintf("expected 'H', got: '%v'", string(lastRune)))
	r.ReadRune()            // Read 'ë' 2 bytes
	r.ReadRune()            // Read 'l'
	lastRune = r.ReadRune() // Read 'l'
	req.Equal('l', lastRune, fmt.Sprintf("expected 'l', got: '%v'", string(lastRune)))
	req.Equal(5, r.Info.ByteOffset)
	req.Equal(4, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	r.UnreadRune() // Unread 'l'
	req.Equal(4, r.Info.ByteOffset)
	req.Equal(3, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	lastRune = r.ReadRune() // Read 'l'
	req.Equal('l', lastRune, fmt.Sprintf("expected 'l', got: '%v'", string(lastRune)))
	req.Equal(5, r.Info.ByteOffset)
	req.Equal(4, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	lastRune = r.ReadRune() // Read 'o'
	req.Equal('o', lastRune, fmt.Sprintf("expected 'o', got: '%v'", string(lastRune)))
	req.Equal(6, r.Info.ByteOffset)
	req.Equal(5, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	lastRune = r.ReadRune() // Read '\n'
	req.Equal('\n', lastRune, fmt.Sprintf("expected '\\n', got: '%v'", string(lastRune)))
	req.Equal(7, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	r.UnreadRune() // Unread '\n'
	req.Equal(6, r.Info.ByteOffset)
	req.Equal(5, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	lastRune = r.ReadRune() // Read '\n'
	req.Equal('\n', lastRune, fmt.Sprintf("expected '\\n', got: '%v'", string(lastRune)))
	req.Equal(7, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	lastRune = r.ReadRune() // Read 'W'
	req.Equal('W', lastRune, fmt.Sprintf("expected 'W', got: '%v'", string(lastRune)))
	req.Equal(8, r.Info.ByteOffset)
	req.Equal(1, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	r.ReadRune()            // Read 'ö'
	r.ReadRune()            // Read 'r'
	r.ReadRune()            // Read 'l'
	lastRune = r.ReadRune() // Read 'd'
	req.Equal('d', lastRune, fmt.Sprintf("expected 'd', got: '%v'", string(lastRune)))
	req.Equal(13, r.Info.ByteOffset)
	req.Equal(5, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	lastRune = r.ReadRune() // Read 'EOF'
	req.Equal(EOF, lastRune, fmt.Sprintf("expected 'EOF', got: '%v'", string(lastRune)))
	req.Equal(14, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(2, r.Info.Line)
}

func TestReader_UnreadRuneRN(t *testing.T) {
	r := New(bufio.NewReader(strings.NewReader("Hëllo\r\nWörld")))
	req := require.New(t)

	// Read the first line
	lastRune := r.ReadRune() // Read 'H'
	req.Equal('H', lastRune, fmt.Sprintf("expected 'H', got: '%v'", string(lastRune)))
	r.ReadRune()            // Read 'ë' 2 bytes
	r.ReadRune()            // Read 'l'
	lastRune = r.ReadRune() // Read 'l'
	req.Equal('l', lastRune, fmt.Sprintf("expected 'l', got: '%v'", string(lastRune)))
	req.Equal(5, r.Info.ByteOffset)
	req.Equal(4, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	r.UnreadRune() // Unread 'l'
	req.Equal(4, r.Info.ByteOffset)
	req.Equal(3, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	lastRune = r.ReadRune() // Read 'l'
	req.Equal('l', lastRune, fmt.Sprintf("expected 'l', got: '%v'", string(lastRune)))
	req.Equal(5, r.Info.ByteOffset)
	req.Equal(4, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	lastRune = r.ReadRune() // Read 'o'
	req.Equal('o', lastRune, fmt.Sprintf("expected 'o', got: '%v'", string(lastRune)))
	req.Equal(6, r.Info.ByteOffset)
	req.Equal(5, r.Info.LineOffset)
	req.Equal(0, r.Info.Line)

	lastRune = r.ReadRune() // Read '\n'
	req.Equal('\r', lastRune, fmt.Sprintf("expected '\\r', got: '%v'", string(lastRune)))
	req.Equal(7, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	lastRune = r.ReadRune() // Read '\n'
	req.Equal('\n', lastRune, fmt.Sprintf("expected '\\n', got: '%v'", string(lastRune)))
	req.Equal(8, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	r.UnreadRune() // Unread '\n'
	req.Equal(7, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	lastRune = r.ReadRune() // Read '\n'
	req.Equal('\n', lastRune, fmt.Sprintf("expected '\\n', got: '%v'", string(lastRune)))
	req.Equal(8, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	lastRune = r.ReadRune() // Read 'W'
	req.Equal('W', lastRune, fmt.Sprintf("expected 'W', got: '%v'", string(lastRune)))
	req.Equal(9, r.Info.ByteOffset)
	req.Equal(1, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	r.ReadRune()            // Read 'ö'
	r.ReadRune()            // Read 'r'
	r.ReadRune()            // Read 'l'
	lastRune = r.ReadRune() // Read 'd'
	req.Equal('d', lastRune, fmt.Sprintf("expected 'd', got: '%v'", string(lastRune)))
	req.Equal(14, r.Info.ByteOffset)
	req.Equal(5, r.Info.LineOffset)
	req.Equal(1, r.Info.Line)

	lastRune = r.ReadRune() // Read 'EOF'
	req.Equal(EOF, lastRune, fmt.Sprintf("expected 'EOF', got: '%v'", string(lastRune)))
	req.Equal(15, r.Info.ByteOffset)
	req.Equal(0, r.Info.LineOffset)
	req.Equal(2, r.Info.Line)
}
