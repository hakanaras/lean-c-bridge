import Lake
open System Lake DSL

package LeanCBridgeTest where
  buildType := .debug

target glue.o pkg : FilePath := do
  let srcJob <- inputFile (pkg.dir / "Test_glue.c") true
  let flags := #["-fPIC", "-I" ++ (<- getLeanIncludeDir).toString, "-I../.."]
  buildO (pkg.buildDir / "glue.o") srcJob flags

extern_lib LibTest pkg := do
  let ffiO <- fetch <| pkg.target ``glue.o
  buildStaticLib (pkg.buildDir / "lib" / nameToStaticLib "lean_libtest") #[ffiO]

lean_lib Test where

@[default_target]
lean_exe Main where
