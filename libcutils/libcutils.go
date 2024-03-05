package libcutils

import (
	"fmt"
	"regexp"
)

// PrintfSpecifier represents the components of a printf format specifier
type PrintfSpecifier struct {
	Original  string // The original specifier string
	Flags     string // Flags: '-', '+', ' ', '#', and '0'
	Width     string // Width: number or '*'
	Precision string // Precision: '.number' or '.*'
	Length    string // Length modifier: 'h', 'hh', 'l', 'll', 'L', 'j', 'z', 't'
	Specifier string // Conversion specifier: 'd', 'i', 'o', 'u', 'x', 'X', 'f', 'F', 'e', 'E', 'g', 'G', 'a', 'A', 'c', 's', 'p', 'n', '%'
}

// ParsePrintfFmt parses a libc printf format string.
func ParsePrintfFmt(fmtStr string) ([]PrintfSpecifier, error) {
	// This regular expression attempts to match all components of printf format specifiers
	re := regexp.MustCompile(`%([-+#0 ]*)(\d+|\*)?(?:\.(\d+|\*))?([hlLjzt]*)([diuoxXfFeEgGaAcspn%])`)
	matches := re.FindAllStringSubmatch(fmtStr, -1)

	if matches == nil {
		return nil, fmt.Errorf("no format specifiers found")
	}

	specifiers := make([]PrintfSpecifier, 0, len(matches))
	for _, match := range matches {
		specifiers = append(specifiers, PrintfSpecifier{
			Original:  match[0],
			Flags:     match[1],
			Width:     match[2],
			Precision: match[3],
			Length:    match[4],
			Specifier: match[5],
		})
	}

	return specifiers, nil
}
