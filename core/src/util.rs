pub fn slice_to_bytes<T>(slice: &[T]) -> &[u8] {
    let size = std::mem::size_of::<T>();
    let len = slice.len() * size;
    let ptr = slice.as_ptr() as *const u8;
    unsafe { std::slice::from_raw_parts(ptr, len) }
}