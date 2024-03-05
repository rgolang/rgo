package llvm

import (
	"fmt"

	"github.com/kr/pretty"
)

func d(v any) string {
	return fmt.Sprintf("%# v", pretty.Formatter(v))
}
