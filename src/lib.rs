use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn, ItemStruct, Path};

// ---------------------------------------------------------------------------
// #[module(Source)] / #[module(Tracker)]
// ---------------------------------------------------------------------------

#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;

    let module_type = parse_macro_input!(attr as Path);
    let module_type_str = module_type
        .segments
        .last()
        .unwrap()
        .ident
        .to_string();

    let expanded = match module_type_str.as_str() {
        "Source" => quote! {
            #input

            #[wasm_export]
            pub fn discover() -> Result<Vec<DiscoverSection>, ChoutenError> {
                #struct_name.discover()
            }
        },

        "Tracker" => quote! {
            #input

            // Returns a pointer to a null-terminated UTF-8 string.
            // ABI: () -> i32  (wasm32 pointer)
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn auth_url() -> i32 {
                static mut AUTH_URL_BUF: [u8; 4096] = [0u8; 4096];
                let s: &str = #struct_name.auth_url("", "", "");
                let bytes = s.as_bytes();
                let len = bytes.len().min(AUTH_URL_BUF.len() - 1);
                AUTH_URL_BUF[..len].copy_from_slice(&bytes[..len]);
                AUTH_URL_BUF[len] = 0;
                AUTH_URL_BUF.as_ptr() as i32
            }

            // ABI: () -> i32
            // Returns pointer to JSON-encoded Result<(), ChoutenError>.
            #[wasm_export]
            pub fn handle_callback() -> Result<(), ChoutenError> {
                let code = __chouten_read_str_arg();
                #struct_name.handle_callback(&code)
            }

            #[wasm_export]
            pub fn refresh_token() -> Result<(), ChoutenError> {
                #struct_name.refresh_token()
            }

            #[wasm_export]
            pub fn discover() -> Result<Vec<DiscoverSection>, ChoutenError> {
                #struct_name.discover()
            }
        },

        _ => panic!("Unsupported module type for #[module(...)]: {}", module_type_str),
    };

    TokenStream::from(expanded)
}

// ---------------------------------------------------------------------------
// #[wasm_export]
//
// Strict ABI rules:
//   - Wrapper always returns i32 (the wasm32 pointer), never *const u8.
//   - No Result, no bool, no usize at the ABI boundary.
//   - No parameters — args are passed via host-provided read functions.
//   - Single exit point, no diverging branches in the wrapper body.
// ---------------------------------------------------------------------------

#[proc_macro_attribute]
pub fn wasm_export(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name  = &input_fn.sig.ident;
    let wrap_name = format_ident!("{}_impl", fn_name);

    let user_fn = &input_fn;

    let expanded = quote! {
        #user_fn

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #wrap_name() -> u32 {
            #[inline(never)]
            fn inner() -> i32 {
                let result = #fn_name();
                let json = ::relay_core::runtime_support::serialize_to_json(&result);
                let len = json.len() as u32;
                let ptr = json.as_ptr() as u32;
                ::core::mem::forget(json);
                let response = ::alloc::boxed::Box::new([ptr, len]);
                ::alloc::boxed::Box::into_raw(response) as i32
            }
            inner() as u32
        }
    };

    TokenStream::from(expanded)
}