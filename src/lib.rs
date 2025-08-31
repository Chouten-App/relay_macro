use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn wasm_export(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let wrapper_name = format_ident!("{}_wrapper", fn_name);

    let user_fn = &input_fn;

    let expanded = quote! {
        use alloc::boxed::Box;
        use alloc::string::String;
        use alloc::vec::Vec;
        use serde_json;

        // Original user function
        #user_fn

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #wrapper_name() -> *const u8 {
            let result = #fn_name();

            // Serialize to JSON string
            unsafe {  core::arch::wasm32::memory_grow(0, 3) };
            let mut json = serde_json::to_string(&result).expect("Failed to serialize");

            // Add null terminator
            json.push('\0');

            // Convert to bytes
            let bytes = json.into_bytes();

            let ptr = bytes.as_ptr();

            // Leak Vec<u8> so it won't be dropped
            core::mem::forget(bytes);

            ptr
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn free_cstring(ptr: *mut u8, len: usize) {
            unsafe {
                drop(Vec::from_raw_parts(ptr, len, len));
            }
        }
    };

    TokenStream::from(expanded)
}
