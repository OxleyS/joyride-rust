// Create a heap-stored array without allocating the array on the stack first (which could overflow it)
// Thanks to r/rust for this code
#[macro_export]
macro_rules! boxed_array {
    ($val:expr ; $len:expr) => {{
        // Use a generic function so that the pointer cast remains type-safe
        fn vec_to_boxed_array<T>(vec: Vec<T>) -> Box<[T; $len]> {
            // Creates a slice, but does not annotate it with its const size
            let boxed_slice = vec.into_boxed_slice();

            // Attach the size annotation by yoinking the pointer, casting, and re-boxing.
            // This does not incur any allocation or copying
            let ptr = ::std::boxed::Box::into_raw(boxed_slice) as *mut [T; $len];
            unsafe { Box::from_raw(ptr) }
        }

        vec_to_boxed_array(vec![$val; $len])
    }};
}
