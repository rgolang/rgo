package ast

import (
	"encoding/json"
	"fmt"
	"io"
	"reflect"
	"strings"

	"github.com/rgolang/rgo/lex"
)

func ToAst(input io.ReadSeeker) ([]byte, error) {
	lexer := lex.New(input)
	parser := New(lexer)

	ast, err := parser.Parse()
	if err != nil {
		return nil, fmt.Errorf("error parsing ast: %w", err)
	}
	b, err := formatAst(ast)
	if err != nil {
		return nil, fmt.Errorf("error json marshalling ast: %w", err)
	}
	return b, nil
}

func formatAst(v any) ([]byte, error) {
	return json.MarshalIndent(processAny(reflect.ValueOf(v), func(m map[string]any, v reflect.Value) {
		m["_type"] = v.Type().Name()
		delete(m, "info")
		if v, ok := m["name"]; ok {
			m["_name"] = v
			delete(m, "name")
		}
		if _, ok := m["token"]; ok {
			delete(m, "token")
		}
		if v, ok := m["id"]; ok {
			m["_id"] = v
			delete(m, "id")
		}
		if v, ok := m["body"]; ok {
			m["inner"] = v // Using "inner" just for the sorting, so it comes after "head"
			delete(m, "body")
		}
	}), " ", "    ")
}

func resolvePointer(val reflect.Value) reflect.Value {
	for val.Kind() == reflect.Ptr {
		if !val.IsValid() || val.IsNil() {
			break
		}
		val = val.Elem()
	}
	return val
}

func processAny(v reflect.Value, opts ...func(map[string]any, reflect.Value)) any {
	switch v.Kind() {
	case reflect.Slice:
		if v.IsNil() {
			return nil
		}

		sliceLen := v.Len()
		resultSlice := make([]any, 0, sliceLen)

		for i := 0; i < sliceLen; i++ {
			elem := v.Index(i)
			elemValue := processAny(elem, opts...)
			if elemValue != nil {
				resultSlice = append(resultSlice, elemValue)
			}
		}

		return resultSlice
	case reflect.Ptr:
		if v.IsNil() {
			return nil
		}
		return processAny(resolvePointer(v.Elem()), opts...)
	case reflect.Struct:
		return processStruct(v, opts...)
	case reflect.Map:
		return processMap(v, opts...)
	case reflect.Interface:
		// If the value is an interface, get its concrete value
		if v.IsNil() {
			return nil
		}
		return processAny(v.Elem(), opts...)
	default:
		if !v.IsValid() {
			return nil
		}
		return v.Interface()
	}
}

func processStruct(v reflect.Value, opts ...func(map[string]any, reflect.Value)) any {
	res := map[string]any{}
	if !v.IsValid() {
		return nil // Return nil or any other suitable zero value
	}

	for i := 0; i < v.NumField(); i++ {
		field := v.Field(i)
		fieldName := strings.ToLower(v.Type().Field(i).Name)

		// skip private fields
		if !field.CanInterface() {
			continue
		}

		// Check if field is a boolean and is false
		if field.Kind() == reflect.Bool && !field.Bool() {
			continue
		}

		// Check if field is a string and is empty
		if field.Kind() == reflect.String && field.Len() == 0 {
			continue
		}

		// skip fields starting with "ir"
		if strings.HasPrefix(fieldName, "ir") {
			continue
		}

		// Skip empty slices
		if field.Kind() == reflect.Slice && field.Len() == 0 {
			continue
		}

		// Skip zero structs
		if field.Kind() == reflect.Struct && field.IsZero() {
			continue
		}

		fieldValue := processAny(field, opts...)
		if fieldValue == nil {
			continue
		}
		res[fieldName] = fieldValue
	}

	for _, opt := range opts {
		opt(res, v)
	}

	return res
}

func processMap(v reflect.Value, opts ...func(map[string]any, reflect.Value)) any {
	if v.IsNil() {
		return nil
	}

	resultMap := make(map[string]any)
	for _, key := range v.MapKeys() {
		val := v.MapIndex(key)
		resultMap[key.Interface().(string)] = processAny(val, opts...)
	}

	for _, opt := range opts {
		opt(resultMap, v)
	}

	return resultMap
}
