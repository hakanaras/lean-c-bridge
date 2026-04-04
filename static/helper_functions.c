lean_obj_res lean_tuple_prepend(lean_obj_res acc, lean_obj_res value) {
    if (acc == lean_box(0)) {
        return value;
    } else {
        lean_obj_res new_tuple = lean_alloc_ctor(0, 2, 0);
        lean_ctor_set(new_tuple, 0, value);
        lean_ctor_set(new_tuple, 1, acc);
        return new_tuple;
    }
}