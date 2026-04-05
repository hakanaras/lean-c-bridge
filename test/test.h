#include <assert.h>
#include <limits.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

// -- Primitive Return Values --

static inline bool bool_return_false() {
    return false;
}

static inline bool bool_return_true() {
    return true;
}

static inline char char_return_min() {
    return CHAR_MIN;
}

static inline char char_return_max() {
    return CHAR_MAX;
}

static inline char char_return_a() {
    return 'a';
}

static inline signed char signed_char_return_min() {
    return SCHAR_MIN;
}

static inline signed char signed_char_return_max() {
    return SCHAR_MAX;
}

static inline unsigned char unsigned_char_return_min() {
    return 0;
}

static inline unsigned char unsigned_char_return_max() {
    return UCHAR_MAX;
}

static inline signed short signed_short_return_min() {
    return SHRT_MIN;
}

static inline signed short signed_short_return_max() {
    return SHRT_MAX;
}

static inline unsigned short unsigned_short_return_min() {
    return 0;
}

static inline unsigned short unsigned_short_return_max() {
    return USHRT_MAX;
}

static inline signed int signed_int_return_min() {
    return INT_MIN;
}

static inline signed int signed_int_return_max() {
    return INT_MAX;
}

static inline unsigned int unsigned_int_return_min() {
    return 0;
}

static inline unsigned int unsigned_int_return_max() {
    return UINT_MAX;
}

static inline signed long signed_long_return_min() {
    return LONG_MIN;
}

static inline signed long signed_long_return_max() {
    return LONG_MAX;
}

static inline unsigned long unsigned_long_return_min() {
    return 0;
}

static inline unsigned long unsigned_long_return_max() {
    return ULONG_MAX;
}

static inline signed long long signed_long_long_return_min() {
    return LLONG_MIN;
}

static inline signed long long signed_long_long_return_max() {
    return LLONG_MAX;
}

static inline unsigned long long unsigned_long_long_return_min() {
    return 0;
}

static inline unsigned long long unsigned_long_long_return_max() {
    return ULLONG_MAX;
}

static inline size_t size_t_return_min() {
    return 0;
}

static inline size_t size_t_return_max() {
    return SIZE_MAX;
}

static inline ptrdiff_t ptrdiff_t_return_min() {
    return PTRDIFF_MIN;
}

static inline ptrdiff_t ptrdiff_t_return_max() {
    return PTRDIFF_MAX;
}

static inline float float_return() {
    return 0.25f;
}

static inline double double_return() {
    return 0.25;
}

static inline long double long_double_return() {
    return 0.25;
}

// -- Primitive Argument Values --

static inline void test_bool_false(bool b) {
    assert(b == false);
}

static inline void test_bool_true(bool b) {
    assert(b == true);
}

static inline void test_char_min(char c) {
    assert(c == CHAR_MIN);
}

static inline void test_char_max(char c) {
    assert(c == CHAR_MAX);
}

static inline void test_char_a(char c) {
    assert(c == 'a');
}

static inline void test_signed_char_min(signed char c) {
    assert(c == SCHAR_MIN);
}

static inline void test_signed_char_max(signed char c) {
    assert(c == SCHAR_MAX);
}

static inline void test_unsigned_char_min(unsigned char c) {
    assert(c == 0);
}

static inline void test_unsigned_char_max(unsigned char c) {
    assert(c == UCHAR_MAX);
}

static inline void test_signed_short_min(signed short s) {
    assert(s == SHRT_MIN);
}

static inline void test_signed_short_max(signed short s) {
    assert(s == SHRT_MAX);
}

static inline void test_unsigned_short_min(unsigned short s) {
    assert(s == 0);
}

static inline void test_unsigned_short_max(unsigned short s) {
    assert(s == USHRT_MAX);
}

static inline void test_signed_int_min(signed int i) {
    assert(i == INT_MIN);
}

static inline void test_signed_int_max(signed int i) {
    assert(i == INT_MAX);
}

static inline void test_unsigned_int_min(unsigned int i) {
    assert(i == 0);
}

static inline void test_unsigned_int_max(unsigned int i) {
    assert(i == UINT_MAX);
}

static inline void test_signed_long_min(signed long l) {
    assert(l == LONG_MIN);
}

static inline void test_signed_long_max(signed long l) {
    assert(l == LONG_MAX);
}

static inline void test_unsigned_long_min(unsigned long l) {
    assert(l == 0);
}

static inline void test_unsigned_long_max(unsigned long l) {
    assert(l == ULONG_MAX);
}

static inline void test_signed_long_long_min(signed long long ll) {
    assert(ll == LLONG_MIN);
}

static inline void test_signed_long_long_max(signed long long ll) {
    assert(ll == LLONG_MAX);
}

static inline void test_unsigned_long_long_min(unsigned long long ll) {
    assert(ll == 0);
}

static inline void test_unsigned_long_long_max(unsigned long long ll) {
    assert(ll == ULLONG_MAX);
}

static inline void test_size_t_min(size_t s) {
    assert(s == 0);
}

static inline void test_size_t_max(size_t s) {
    assert(s == SIZE_MAX);
}

static inline void test_ptrdiff_t_min(ptrdiff_t p) {
    assert(p == PTRDIFF_MIN);
}

static inline void test_ptrdiff_t_max(ptrdiff_t p) {
    assert(p == PTRDIFF_MAX);
}

static inline void test_float(float f) {
    assert(f == 0.25f);
}

static inline void test_double(double d) {
    assert(d == 0.25);
}

static inline void test_long_double(long double ld) {
    assert(ld == 0.25);
}

// -- Non-IO Pass-Through Functions --

static inline bool no_io_bool_pass_through(bool b) {
    return b;
}

static inline char no_io_char_pass_through(char c) {
    return c;
}

static inline signed char no_io_signed_char_pass_through(signed char c) {
    return c;
}

static inline unsigned char no_io_unsigned_char_pass_through(unsigned char c) {
    return c;
}

static inline signed short no_io_signed_short_pass_through(signed short s) {
    return s;
}

static inline unsigned short no_io_unsigned_short_pass_through(unsigned short s) {
    return s;
}

static inline signed int no_io_signed_int_pass_through(signed int i) {
    return i;
}

static inline unsigned int no_io_unsigned_int_pass_through(unsigned int i) {
    return i;
}

static inline signed long no_io_signed_long_pass_through(signed long l) {
    return l;
}

static inline unsigned long no_io_unsigned_long_pass_through(unsigned long l) {
    return l;
}

static inline signed long long no_io_signed_long_long_pass_through(signed long long ll) {
    return ll;
}

static inline unsigned long long no_io_unsigned_long_long_pass_through(unsigned long long ll) {
    return ll;
}

static inline size_t no_io_size_t_pass_through(size_t s) {
    return s;
}

static inline ptrdiff_t no_io_ptrdiff_t_pass_through(ptrdiff_t p) {
    return p;
}

static inline float no_io_float_pass_through(float f) {
    return f;
}

static inline double no_io_double_pass_through(double d) {
    return d;
}

static inline long double no_io_long_double_pass_through(long double ld) {
    return ld;
}

// -- Enums --

typedef enum test_enum {
    TEST_ENUM_VALUE_1 = 1,
    TEST_ENUM_VALUE_2
} test_enum;

static inline test_enum test_enum_return_1() {
    return TEST_ENUM_VALUE_1;
}

static inline test_enum test_enum_return_invalid() {
    return (test_enum) 999;
}

static inline void test_enum_take_2(test_enum e) {
    assert(e == TEST_ENUM_VALUE_2);
}

static inline void test_enum_take_invalid(test_enum e) {
    assert(e == (test_enum) 999);
}

// -- Static Array --

typedef struct static_array_struct {
    int arr[4];
} static_array_struct;

static inline static_array_struct static_array_return() {
    static_array_struct s = {
        .arr = {1, 2, 3, 4}
    };
    return s;
}

static inline void static_array_take(int arr[4]) {
    assert(arr[0] == 1);
    assert(arr[1] == 2);
    assert(arr[2] == 3);
    assert(arr[3] == 4);
}

// -- Dynamic Array --

static inline void dynamic_array_take(int* array, size_t length, int length_taken) {
    if (length_taken < 0) {
        assert(length == 0);
        assert(array == NULL);
    } else {
        assert(length == (size_t)length_taken);
        for (size_t i = 0; i < length; i++) {
            assert(array[i] == (int)(i + 1));
        }
    }
}

static inline void dynamic_string_array_take(char** array, size_t length) {
    assert(length == 2);
    assert(strcmp(array[0], "Hello") == 0);
    assert(strcmp(array[1], "World") == 0);
    assert(array[2] == NULL);
}

static inline char** dynamic_string_array_return() {
    char** arr = malloc(3 * sizeof(char*));
    arr[0] = "Hello";
    arr[1] = "World";
    arr[2] = NULL;
    return arr;
}

static inline char** dynamic_string_array_return_nullable(int length) {
    if (length < 0) {
        return NULL;
    } else {
        char** arr = malloc(((size_t)length + 1) * sizeof(char*));
        for (int i = 0; i < length; i++) {
            arr[i] = "Foo";
        }
        arr[length] = NULL;
        return arr;
    }
}

static inline char** array_and_string_return(char* str_buffer, size_t buffer_size) {
    return dynamic_string_array_return();
}

// -- Strings --

static inline char* lit_string_return() {
    return "Hello, World!";
}

static inline char* alloc_string_return() {
    char* str = malloc(14);
    strcpy(str, "Hello, World!");
    return str;
}

static inline char* alloc_string_return_nullable(int length) {
    if (length < 0) {
        return NULL;
    } else {
        char* str = malloc((size_t)length + 1);
        strncpy(str, "Hello, World!", (size_t) length);
        str[length] = '\0';
        return str;
    }
}

static inline void string_take(const char* str) {
    assert(strcmp(str, "Hello, World!") == 0);
}

static inline void string_take_with_length(const char* str, size_t length) {
    assert(length == 13);
    assert(strncmp(str, "Hello, World!", length) == 0);
}

static inline void string_take_null(const char* str) {
    assert(str == NULL);
}

// -- Structs --

typedef struct int_values {
    bool bool_false;
    bool bool_true;
    char char_min;
    char char_max;
    char char_a;
    signed char signed_char_min;
    signed char signed_char_max;
    unsigned char unsigned_char_min;
    unsigned char unsigned_char_max;
    signed short signed_short_min;
    signed short signed_short_max;
    unsigned short unsigned_short_min;
    unsigned short unsigned_short_max;
    signed int signed_int_min;
    signed int signed_int_max;
    unsigned int unsigned_int_min;
    unsigned int unsigned_int_max;
    signed long signed_long_min;
    signed long signed_long_max;
    unsigned long unsigned_long_min;
    unsigned long unsigned_long_max;
    signed long long signed_long_long_min;
    signed long long signed_long_long_max;
    unsigned long long unsigned_long_long_min;
    unsigned long long unsigned_long_long_max;
    size_t size_t_min;
    size_t size_t_max;
    ptrdiff_t ptrdiff_t_min;
    ptrdiff_t ptrdiff_t_max;
} int_values;

typedef struct float_values {
    float float_value;
    double double_value;
    long double long_double_value;
} float_values;

typedef struct test_values {
    int_values ints;
    float_values floats;
} test_values;

static inline void set_struct_values(test_values* values) {
    values->ints.bool_false = false;
    values->ints.bool_true = true;
    values->ints.char_min = CHAR_MIN;
    values->ints.char_max = CHAR_MAX;
    values->ints.char_a = 'a';
    values->ints.signed_char_min = SCHAR_MIN;
    values->ints.signed_char_max = SCHAR_MAX;
    values->ints.unsigned_char_min = 0;
    values->ints.unsigned_char_max = UCHAR_MAX;
    values->ints.signed_short_min = SHRT_MIN;
    values->ints.signed_short_max = SHRT_MAX;
    values->ints.unsigned_short_min = 0;
    values->ints.unsigned_short_max = USHRT_MAX;
    values->ints.signed_int_min = INT_MIN;
    values->ints.signed_int_max = INT_MAX;
    values->ints.unsigned_int_min = 0;
    values->ints.unsigned_int_max = UINT_MAX;
    values->ints.signed_long_min = LONG_MIN;
    values->ints.signed_long_max = LONG_MAX;
    values->ints.unsigned_long_min = 0;
    values->ints.unsigned_long_max = ULONG_MAX;
    values->ints.signed_long_long_min = LLONG_MIN;
    values->ints.signed_long_long_max = LLONG_MAX;
    values->ints.unsigned_long_long_min = 0;
    values->ints.unsigned_long_long_max = ULLONG_MAX;
    values->ints.size_t_min = 0;
    values->ints.size_t_max = SIZE_MAX;
    values->ints.ptrdiff_t_min = PTRDIFF_MIN;
    values->ints.ptrdiff_t_max = PTRDIFF_MAX;
    values->floats.float_value = 0.25f;
    values->floats.double_value = 0.25;
    values->floats.long_double_value = 0.25;
}

static inline test_values struct_return() {
    test_values values;
    set_struct_values(&values);
    return values;
}

static inline void struct_take(test_values values) {
    test_values expected;
    set_struct_values(&expected);
    assert(values.ints.bool_false == expected.ints.bool_false);
    assert(values.ints.bool_true == expected.ints.bool_true);
    assert(values.ints.char_min == expected.ints.char_min);
    assert(values.ints.char_max == expected.ints.char_max);
    assert(values.ints.char_a == expected.ints.char_a);
    assert(values.ints.signed_char_min == expected.ints.signed_char_min);
    assert(values.ints.signed_char_max == expected.ints.signed_char_max);
    assert(values.ints.unsigned_char_min == expected.ints.unsigned_char_min);
    assert(values.ints.unsigned_char_max == expected.ints.unsigned_char_max);
    assert(values.ints.signed_short_min == expected.ints.signed_short_min);
    assert(values.ints.signed_short_max == expected.ints.signed_short_max);
    assert(values.ints.unsigned_short_min == expected.ints.unsigned_short_min);
    assert(values.ints.unsigned_short_max == expected.ints.unsigned_short_max);
    assert(values.ints.signed_int_min == expected.ints.signed_int_min);
    assert(values.ints.signed_int_max == expected.ints.signed_int_max);
    assert(values.ints.unsigned_int_min == expected.ints.unsigned_int_min);
    assert(values.ints.unsigned_int_max == expected.ints.unsigned_int_max);
    assert(values.ints.signed_long_min == expected.ints.signed_long_min);
    assert(values.ints.signed_long_max == expected.ints.signed_long_max);
    assert(values.ints.unsigned_long_min == expected.ints.unsigned_long_min);
    assert(values.ints.unsigned_long_max == expected.ints.unsigned_long_max);
    assert(values.ints.signed_long_long_min == expected.ints.signed_long_long_min);
    assert(values.ints.signed_long_long_max == expected.ints.signed_long_long_max);
    assert(values.ints.unsigned_long_long_min == expected.ints.unsigned_long_long_min);
    assert(values.ints.unsigned_long_long_max == expected.ints.unsigned_long_long_max);
    assert(values.ints.size_t_min == expected.ints.size_t_min);
    assert(values.ints.size_t_max == expected.ints.size_t_max);
    assert(values.ints.ptrdiff_t_min == expected.ints.ptrdiff_t_min);
    assert(values.ints.ptrdiff_t_max == expected.ints.ptrdiff_t_max);
    assert(values.floats.float_value == expected.floats.float_value);
    assert(values.floats.double_value == expected.floats.double_value);
    assert(values.floats.long_double_value == expected.floats.long_double_value);
}

static inline test_values* struct_return_opaque() {
    test_values* values = malloc(sizeof(test_values));
    set_struct_values(values);
    return values;
}

static inline void struct_take_opaque(test_values* values) {
    struct_take(*values);
    free(values);
}

// -- Omit & Static Expr --

static inline void omitted() {
    assert(false);
}

static inline void static_expr_take(int x) {
    assert(x == 42);
}

// -- Out Parameters --

static inline int out_tuple_test(int input, int* doubled, char** message) {
    *doubled = input * 2;
    *message = malloc(10);
    strcpy(*message, "out-value");
    return input + 1;
}

static inline void out_only_test(int* left, int* right) {
    *left = 7;
    *right = 9;
}

static inline void string_buffer_without_length_test(char* buffer) {
    strcpy(buffer, "Hello, World! Hello, World!");
}

static inline void string_buffer_with_length_test(char* buffer, size_t buffer_size) {
    strncpy(buffer, "Hello, World! Hello, World!", buffer_size - 1);
    buffer[buffer_size - 1] = '\0';
}
