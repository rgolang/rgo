// reader normalizes OS newlines and returns EOF for end of line and panics for errors, which simplifies its usage
// it also keeps track of the position of the cursor in terms of bytes, lines and columns.
package reader

import (
	"bufio"
	"io"
	"unicode/utf8"
)

const EOF rune = 3 // Pretend there's an EOF byte at the end of the file to simplify control flow, it's treated like a newline TODO: might be better to use -1

type Info struct {
	prevRune       rune   // The previous read rune
	Line           int    // The line number (start with 0)
	LineRuneOffset int    // The number of runes read on this line
	LineOffset     int    // The number of bytes read on this line
	ByteOffset     int    // The number of bytes read so far
	File           string // The file path if present
}

type Reader struct {
	isEnd    bool
	Info     Info
	PrevInfo Info
	reader   *bufio.Reader
	seeker   io.ReadSeeker
}

func New(readSeeker io.ReadSeeker) *Reader {
	return &Reader{
		reader: bufio.NewReader(readSeeker),
		seeker: readSeeker,
	}
}

func (r *Reader) ReadRune() rune {
	if r.isEnd {
		return EOF
	}
	lastRune, size, err := r.reader.ReadRune()
	if err != nil {
		if err == io.EOF {
			r.isEnd = true
			r.PrevInfo = r.Info
			r.Info.ByteOffset++
			r.Info.LineRuneOffset = 0
			r.Info.LineOffset = 0
			r.Info.Line++
			return EOF
		}
		panic(err)
	}
	r.PrevInfo = r.Info // Store info so that the rune can be unread
	defer func() {
		r.Info.prevRune = lastRune
	}()

	r.Info.ByteOffset += size

	// We just read \r\n, counts as one line
	if r.Info.prevRune == '\r' && lastRune == '\n' {
		return lastRune
	}

	// We just read \r or \n, increment line
	if lastRune == '\r' || lastRune == '\n' {
		r.Info.Line++
		r.Info.LineRuneOffset = 0
		r.Info.LineOffset = 0
		return lastRune
	}

	// Add rune to line offset
	r.Info.LineRuneOffset++
	r.Info.LineOffset += utf8.RuneLen(lastRune)
	return lastRune
}

func (r *Reader) UnreadRune() {
	if !r.isEnd {
		err := r.reader.UnreadRune()
		if err != nil {
			panic(err)
		}
	}
	r.Info = r.PrevInfo // Restore info to previous state
}

func (r *Reader) Seek(offset int64, whence int) (int64, error) {
	r.isEnd = false
	n, err := r.seeker.Seek(offset, whence)
	if err != nil {
		return 0, err
	}
	r.reader = bufio.NewReader(r.seeker)
	return n, nil
}
