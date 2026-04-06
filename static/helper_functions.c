#include <stdint.h>

static inline void lean_ffi_panic_if_null(void * ptr) {
    if (ptr == NULL) {
        lean_internal_panic_out_of_memory();
    }
}

// -- Array and String Helpers --

static inline void lean_ffi_check_allocation_size(size_t element_size, size_t count) {
    if (element_size != 0 && count > SIZE_MAX / element_size) {
        lean_internal_panic_out_of_memory();
    }
}

static inline void * lean_ffi_malloc_array(size_t element_size, size_t count) {
    lean_ffi_check_allocation_size(element_size, count);
    void * ptr = malloc(element_size * count);
    lean_ffi_panic_if_null(ptr);
    return ptr;
}

static inline void * lean_ffi_malloc_array_or_null(size_t element_size, size_t count) {
    if (count == 0) {
        return NULL;
    }
    return lean_ffi_malloc_array(element_size, count);
}

static inline void * lean_ffi_calloc_array(size_t count, size_t element_size) {
    lean_ffi_check_allocation_size(element_size, count);
    void * ptr = calloc(count, element_size);
    lean_ffi_panic_if_null(ptr);
    return ptr;
}

static inline char * lean_ffi_copy_lean_string(lean_obj_arg value, size_t * len_out) {
    size_t bytes = lean_string_size(value) - 1;
    char * buffer = (char *)lean_ffi_malloc_array(sizeof(char), bytes + 1);
    memcpy(buffer, lean_string_cstr(value), bytes + 1);
    if (len_out != NULL) {
        *len_out = bytes;
    }
    return buffer;
}

static inline lean_obj_res lean_ffi_mk_array_with_capacity(size_t capacity) {
    return lean_mk_empty_array_with_capacity(lean_box(capacity));
}

#define LEAN_FFI_MALLOC_ARRAY(type, count) ((type *)lean_ffi_malloc_array(sizeof(type), (count)))
#define LEAN_FFI_MALLOC_ARRAY_OR_NULL(type, count) ((type *)lean_ffi_malloc_array_or_null(sizeof(type), (count)))
#define LEAN_FFI_CALLOC_ARRAY(type, count) ((type *)lean_ffi_calloc_array((count), sizeof(type)))

// -- Tuple Helpers --

static inline lean_obj_res lean_ffi_tuple_prepend(lean_obj_res acc, lean_obj_res value) {
    if (acc == lean_box(0)) {
        return value;
    } else {
        lean_obj_res new_tuple = lean_alloc_ctor(0, 2, 0);
        lean_ctor_set(new_tuple, 0, value);
        lean_ctor_set(new_tuple, 1, acc);
        return new_tuple;
    }
}

// -- Option Helpers --

static inline lean_obj_res lean_ffi_option_none() {
    return lean_alloc_ctor(0, 0, 0);
}

static inline bool lean_ffi_option_is_none(b_lean_obj_arg option) {
    return lean_obj_tag(option) == 0;
}

static inline lean_obj_res lean_ffi_option_some(lean_obj_res value) {
    lean_obj_res some = lean_alloc_ctor(1, 1, 0);
    lean_ctor_set(some, 0, value);
    return some;
}

static inline bool lean_ffi_option_is_some(b_lean_obj_arg option) {
    return lean_obj_tag(option) == 1;
}

static inline b_lean_obj_res lean_ffi_option_get(b_lean_obj_arg option) {
    if (lean_ffi_option_is_none(option)) {
        lean_internal_panic("attempted to get value from none option");
    }
    return lean_ctor_get(option, 0);
}

// -- Byte Array Helpers --

static inline lean_obj_res lean_ffi_mk_byte_array(const uint8_t *data, size_t size) {
    lean_obj_res byte_array = lean_mk_empty_byte_array(lean_box(size));
    for (size_t i = 0; i < size; i++) {
        byte_array = lean_byte_array_push(byte_array, data[i]);
    }
    return byte_array;
}

static inline uint8_t *lean_ffi_byte_array_to_c(b_lean_obj_arg byte_array, size_t *size_out) {
    *size_out = lean_unbox(lean_byte_array_size(byte_array));
    uint8_t *data = LEAN_FFI_MALLOC_ARRAY_OR_NULL(uint8_t, *size_out);
    for (size_t i = 0; i < *size_out; i++) {
        data[i] = lean_byte_array_uget(byte_array, i);
    }
    return data;
}
