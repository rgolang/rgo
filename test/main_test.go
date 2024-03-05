package main_test

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"
)

func compile(filename string) error {
	// strip extension from filename
	ext := filepath.Ext(filename)
	name := strings.TrimSuffix(filename, ext)
	commands := []string{
		"go run ../main.go " + name + ".rgo",
		"llc -filetype=obj -relocation-model=pic " + name + ".ll",
		"gcc " + name + ".o -o ./" + name,
	}
	for _, cmd := range commands {
		cmd := exec.Command("sh", "-c", cmd)
		output, err := cmd.CombinedOutput()
		if err != nil {
			return fmt.Errorf("error running command %q: %s, output: %s", cmd, err, output)
		}
	}

	// Remove .ll and .o files
	err := os.Remove(name + ".ll")
	if err != nil {
		return fmt.Errorf("error removing file %q: %s", name+".ll", err)
	}

	err = os.Remove(name + ".o")
	if err != nil {
		return fmt.Errorf("error removing file %q: %s", name+".o", err)
	}

	return nil
}

func run(command string) (string, error) {
	cmd := exec.Command(command)
	// Get the output of the command
	output, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("error running command %q: %s, output: %s", cmd, err, output)
	}
	return string(output), nil
}

func compileAndRun(t *testing.T, name string) string {
	t.Helper()
	file := name + ".rgo"
	err := compile(file)
	require.NoError(t, err)
	out, err := run("./" + name)
	require.NoError(t, err)
	err = os.Remove(name)
	if err != nil {
		panic(fmt.Sprintf("error removing file %q: %s", name, err))
	}
	return out
}

func TestHelloWorld(t *testing.T) {
	require.Equal(t, "Hello, world!\n\n", compileAndRun(t, "hello"))
}

func TestHelloWorldConst(t *testing.T) {
	require.Equal(t, "Hello world!\n\n", compileAndRun(t, "helloconst"))
}

func TestHelloWorldFunc(t *testing.T) {
	require.Equal(t, "hello world\nhello world\n", compileAndRun(t, "hellofunc"))
}

func TestHelloWorldQualifiedFunc(t *testing.T) { // TODO: make these tests work without using main.go to leverage test caching (and re-enable it)
	require.Equal(t, "Hello world!\n\n", compileAndRun(t, "helloqualified")) // TODO: join these into one big test
}

func TestHelloWorldQualifiedFunc2(t *testing.T) {
	require.Equal(t, "Hello world!\n\n", compileAndRun(t, "helloqualified2"))
}

func TestHelloInt(t *testing.T) {
	require.Equal(t, "Hello 42!", compileAndRun(t, "helloint"))
}

func TestHelloCurryString(t *testing.T) {
	require.Equal(t, "The winning number for Alice is 42\nThe winning number for Bob is 43\n", compileAndRun(t, "hellocurrystr"))
}

func TestInternalCallbackGenerics(t *testing.T) {
	t.Skip()
	require.Equal(t, "The losing number is 41\nThe winning number is 42\n", compileAndRun(t, "callbackgenerics"))
}

func TestCallback1(t *testing.T) {
	require.Equal(t, "hello world\n", compileAndRun(t, "hellocallback1"))
}

func TestCallback2(t *testing.T) {
	require.Equal(t, "The winning number is 42\n", compileAndRun(t, "hellocallback2"))
}

func TestAppliedCallback(t *testing.T) {
	require.Equal(t, "msg1: hi, msg2: bye\n", compileAndRun(t, "helloappliedcallback"))
}
