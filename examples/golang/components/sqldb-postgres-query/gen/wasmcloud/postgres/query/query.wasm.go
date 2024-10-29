// Code generated by wit-bindgen-go. DO NOT EDIT.

package query

import (
	"github.com/bytecodealliance/wasm-tools-go/cm"
)

// This file contains wasmimport and wasmexport declarations for "wasmcloud:postgres@0.1.1-draft".

//go:wasmimport wasmcloud:postgres/query@0.1.1-draft query
//go:noescape
func wasmimport_Query(query0 *uint8, query1 uint32, params0 *PgValue, params1 uint32, result *cm.Result[QueryErrorShape, cm.List[ResultRow], QueryError])

//go:wasmimport wasmcloud:postgres/query@0.1.1-draft query-batch
//go:noescape
func wasmimport_QueryBatch(query0 *uint8, query1 uint32, result *cm.Result[QueryError, struct{}, QueryError])
