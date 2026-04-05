import Test

open Test

-- Test that `omitted` was omitted:
def Test.omitted : Unit := ()

def test_primitive_returns : IO Unit := do
  assert! (<- bool_return_false) == 0
  assert! (<- bool_return_true) == 1
  assert! (<- char_return_min) == -128
  assert! (<- char_return_max) == 127
  assert! (<- char_return_a) == 97
  assert! (<- signed_char_return_min) == -128
  assert! (<- signed_char_return_max) == 127
  assert! (<- unsigned_char_return_min) == 0
  assert! (<- unsigned_char_return_max) == 255
  assert! (<- signed_short_return_min) == -32768
  assert! (<- signed_short_return_max) == 32767
  assert! (<- unsigned_short_return_min) == 0
  assert! (<- unsigned_short_return_max) == 65535
  assert! (<- signed_int_return_min) == -2147483648
  assert! (<- signed_int_return_max) == 2147483647
  assert! (<- unsigned_int_return_min) == 0
  assert! (<- unsigned_int_return_max) == 4294967295
  assert! (<- signed_long_return_min) == -9223372036854775808
  assert! (<- signed_long_return_max) == 9223372036854775807
  assert! (<- unsigned_long_return_min) == 0
  assert! (<- unsigned_long_return_max) == 18446744073709551615
  assert! (<- signed_long_long_return_min) == -9223372036854775808
  assert! (<- signed_long_long_return_max) == 9223372036854775807
  assert! (<- unsigned_long_long_return_min) == 0
  assert! (<- unsigned_long_long_return_max) == 18446744073709551615
  assert! (<- size_t_return_min) == 0
  assert! (<- size_t_return_max) == 18446744073709551615
  assert! (<- ptrdiff_t_return_min) == -9223372036854775808
  assert! (<- ptrdiff_t_return_max) == 9223372036854775807
  assert! (<- float_return) == 0.25
  assert! (<- double_return) == 0.25
  assert! (<- long_double_return) == 0.25

def test_primitive_arguments : IO Unit := do
  test_bool_false 0
  test_bool_true 1
  test_char_min (-128)
  test_char_max 127
  test_char_a 97
  test_signed_char_min (-128)
  test_signed_char_max 127
  test_unsigned_char_min 0
  test_unsigned_char_max 255
  test_signed_short_min (-32768)
  test_signed_short_max 32767
  test_unsigned_short_min 0
  test_unsigned_short_max 65535
  test_signed_int_min (-2147483648)
  test_signed_int_max 2147483647
  test_unsigned_int_min 0
  test_unsigned_int_max 4294967295
  test_signed_long_min (-9223372036854775808)
  test_signed_long_max 9223372036854775807
  test_unsigned_long_min 0
  test_unsigned_long_max 18446744073709551615
  test_signed_long_long_min (-9223372036854775808)
  test_signed_long_long_max 9223372036854775807
  test_unsigned_long_long_min 0
  test_unsigned_long_long_max 18446744073709551615
  test_size_t_min 0
  test_size_t_max 18446744073709551615
  test_ptrdiff_t_min (-9223372036854775808)
  test_ptrdiff_t_max 9223372036854775807
  test_float 0.25
  test_double 0.25
  test_long_double 0.25

def test_no_io_pass_through : IO Unit := do
  assert! (no_io_bool_pass_through 0) == 0
  assert! (no_io_bool_pass_through 1) == 1
  assert! (no_io_char_pass_through (-128)) == -128
  assert! (no_io_char_pass_through 127) == 127
  assert! (no_io_char_pass_through 97) == 97
  assert! (no_io_signed_char_pass_through (-128)) == -128
  assert! (no_io_signed_char_pass_through 127) == 127
  assert! (no_io_signed_char_pass_through 1) == 1
  assert! (no_io_unsigned_char_pass_through 0) == 0
  assert! (no_io_unsigned_char_pass_through 255) == 255
  assert! (no_io_unsigned_char_pass_through 1) == 1
  assert! (no_io_signed_short_pass_through (-32768)) == -32768
  assert! (no_io_signed_short_pass_through 32767) == 32767
  assert! (no_io_signed_short_pass_through 1) == 1
  assert! (no_io_unsigned_short_pass_through 0) == 0
  assert! (no_io_unsigned_short_pass_through 65535) == 65535
  assert! (no_io_unsigned_short_pass_through 1) == 1
  assert! (no_io_signed_int_pass_through (-2147483648)) == -2147483648
  assert! (no_io_signed_int_pass_through 2147483647) == 2147483647
  assert! (no_io_signed_int_pass_through 1) == 1
  assert! (no_io_unsigned_int_pass_through 0) == 0
  assert! (no_io_unsigned_int_pass_through 4294967295) == 4294967295
  assert! (no_io_unsigned_int_pass_through 1) == 1
  assert! (no_io_signed_long_pass_through (-9223372036854775808)) == -9223372036854775808
  assert! (no_io_signed_long_pass_through 9223372036854775807) == 9223372036854775807
  assert! (no_io_signed_long_pass_through 1) == 1
  assert! (no_io_unsigned_long_pass_through 0) == 0
  assert! (no_io_unsigned_long_pass_through 18446744073709551615) == 18446744073709551615
  assert! (no_io_unsigned_long_pass_through 1) == 1
  assert! (no_io_signed_long_long_pass_through (-9223372036854775808)) == -9223372036854775808
  assert! (no_io_signed_long_long_pass_through 9223372036854775807) == 9223372036854775807
  assert! (no_io_signed_long_long_pass_through 1) == 1
  assert! (no_io_unsigned_long_long_pass_through 0) == 0
  assert! (no_io_unsigned_long_long_pass_through 18446744073709551615) == 18446744073709551615
  assert! (no_io_unsigned_long_long_pass_through 1) == 1
  assert! (no_io_size_t_pass_through 0) == 0
  assert! (no_io_size_t_pass_through 18446744073709551615) == 18446744073709551615
  assert! (no_io_size_t_pass_through 1) == 1
  assert! (no_io_ptrdiff_t_pass_through (-9223372036854775808)) == -9223372036854775808
  assert! (no_io_ptrdiff_t_pass_through 9223372036854775807) == 9223372036854775807
  assert! (no_io_ptrdiff_t_pass_through 1) == 1
  assert! (no_io_float_pass_through 0.25) == 0.25
  assert! (no_io_double_pass_through 0.25) == 0.25
  assert! (no_io_long_double_pass_through 0.25) == 0.25

def test_enums : IO Unit := do
  assert! (<- test_enum_return_1) == test_enum.TEST_ENUM_VALUE_1
  assert! (<- test_enum_return_invalid) == test_enum.other 999
  test_enum_take_2 test_enum.TEST_ENUM_VALUE_2
  test_enum_take_invalid (test_enum.other 999)

def test_arrays : IO Unit := do
  assert! (<- static_array_return).arr == #[1, 2, 3, 4]
  dynamic_array_take (.some #[1, 2, 3, 4]) 4
  dynamic_array_take (.some #[]) 0
  dynamic_array_take (.none) (-1)
  dynamic_string_array_take #["Hello", "World"]
  static_array_take #[1, 2, 3, 4]
  assert! (<- dynamic_string_array_return) == #["Hello", "World"]
  assert! (<- dynamic_string_array_return_nullable (-1)) == .none
  assert! (<- dynamic_string_array_return_nullable 0) == .some #[]
  assert! (<- dynamic_string_array_return_nullable 4) == .some #["Foo", "Foo", "Foo", "Foo"]
  let (a2, str) <- array_and_string_return
  assert! a2 == #["Hello", "World"]
  assert! str == ""


def test_structs : IO Unit := do
  let s1 <- struct_return
  assert! s1.ints.bool_false == 0
  assert! s1.ints.bool_true == 1
  assert! s1.ints.char_min == -128
  assert! s1.ints.char_max == 127
  assert! s1.ints.char_a == 97
  assert! s1.ints.signed_char_min == -128
  assert! s1.ints.signed_char_max == 127
  assert! s1.ints.unsigned_char_min == 0
  assert! s1.ints.unsigned_char_max == 255
  assert! s1.ints.signed_short_min == -32768
  assert! s1.ints.signed_short_max == 32767
  assert! s1.ints.unsigned_short_min == 0
  assert! s1.ints.unsigned_short_max == 65535
  assert! s1.ints.signed_int_min == -2147483648
  assert! s1.ints.signed_int_max == 2147483647
  assert! s1.ints.unsigned_int_min == 0
  assert! s1.ints.unsigned_int_max == 4294967295
  assert! s1.ints.signed_long_min == -9223372036854775808
  assert! s1.ints.signed_long_max == 9223372036854775807
  assert! s1.ints.unsigned_long_min == 0
  assert! s1.ints.unsigned_long_max == 18446744073709551615
  assert! s1.ints.signed_long_long_min == -9223372036854775808
  assert! s1.ints.signed_long_long_max == 9223372036854775807
  assert! s1.ints.unsigned_long_long_min == 0
  assert! s1.ints.unsigned_long_long_max == 18446744073709551615
  assert! s1.ints.size_t_min == 0
  assert! s1.ints.size_t_max == 18446744073709551615
  assert! s1.ints.ptrdiff_t_min == -9223372036854775808
  assert! s1.ints.ptrdiff_t_max == 9223372036854775807
  assert! s1.floats.float_value == 0.25
  assert! s1.floats.double_value == 0.25
  assert! s1.floats.long_double_value == 0.25
  struct_take s1
  let s2 <- struct_return_opaque
  struct_take_opaque s2

def test_strings : IO Unit := do
  let s1 <- lit_string_return
  assert! s1 == "Hello, World!"
  let s2 <- alloc_string_return
  assert! s2 == "Hello, World!"
  string_take "Hello, World!"
  string_take_with_length "Hello, World!"
  string_take_null .none
  assert! (<- alloc_string_return_nullable (-1)) == .none
  assert! (<- alloc_string_return_nullable 0) == .some ""
  assert! (<- alloc_string_return_nullable 13) == .some "Hello, World!"

  def test_dereference_returns : IO Unit := do
    assert! (<- dereference_int_return) == 123
    assert! (<- dereference_int_return_nullable (-1)) == .none
    assert! (<- dereference_int_return_nullable 7) == .some 8
    assert! (<- dereference_string_return) == "Dereference"

def test_out_params : IO Unit := do
  let (i1, i2, s) <- out_tuple_test 42
  assert! i1 == 43
  assert! i2 == 84
  assert! s == "out-value"
  let (i1, i2) <- out_only_test
  assert! i1 == 7
  assert! i2 == 9
  assert! (<- string_buffer_without_length_test) == "Hello, World! Hello, World!"
  assert! (<- string_buffer_with_length_test) == "Hello, World!"

def main : IO Unit := do
  test_primitive_returns
  test_primitive_arguments
  test_no_io_pass_through
  test_enums
  test_arrays
  test_structs
  test_strings
  test_dereference_returns
  test_out_params
  static_expr_take

  IO.println "All tests passed!"
