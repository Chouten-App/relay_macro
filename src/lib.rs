use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, ItemFn, ItemStruct, Path};

#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the struct
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;

    // Parse the argument (Source, Tracker, etc.)
    let module_type = parse_macro_input!(attr as Path);
    let module_type_str = module_type.segments.last().unwrap().ident.to_string();

    // Generate different functions depending on module type
    let expanded = match module_type_str.as_str() {
        "Source" => {
            quote! {
                #input

                #[wasm_export]
                pub fn discover() -> Result<Vec<DiscoverSection>, ChoutenError> {
                    #struct_name.discover()
                }
            }
        }

        "Tracker" => {
            quote! {
                #input

                #[unsafe(no_mangle)]
                pub unsafe fn auth_url() -> *const u8 {
                    let bytes = #struct_name.auth_url("", "", "").into_bytes();

                    let ptr = bytes.as_ptr();

                    // Leak Vec<u8> so it won't be dropped
                    core::mem::forget(bytes);

                    ptr
                }

                
                #[unsafe(no_mangle)]
                pub unsafe fn handle_callback(code: &str) -> Result<(), ChoutenError> {
                    #struct_name.handle_callback(code)
                }

                #[wasm_export]
                pub fn refresh_token() -> Result<(), ChoutenError> {
                    #struct_name.refresh_token()
                }

                #[wasm_export]
                pub fn discover() -> Result<Vec<DiscoverSection>, ChoutenError> {
                    #struct_name.discover()
                }
            }
        }

        _ => panic!("Unsupported module type for #[module(...)]"),
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn wasm_export(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let wrapper_name = format_ident!("{}_wrapper", fn_name);

    let user_fn = &input_fn;

    let expanded = quote! {
        // Original user function
        #user_fn

        static mut JSON_BUF: [u8; 4096] = [0; 4096];

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #wrapper_name() -> *const u8 {
            let result = #fn_name();
            
            match serde_json_core::to_slice(&result, &mut JSON_BUF[..]) {
                Ok(len) => {
                    if len < JSON_BUF.len() {
                        JSON_BUF[len] = 0; // null terminator
                    }
                }
                Err(_) => {
                    let err = b"{\"error\":\"overflow\"}\0";
                    JSON_BUF[..err.len()].copy_from_slice(err);
                }
            }
            
            let ptr = JSON_BUF.as_ptr();

            //json.push('\0');

            // Convert to bytes
            //let bytes = json.into_bytes();

            //let ptr = bytes.as_ptr();

            // Leak Vec<u8> so it won't be dropped

            ptr
        }
    };

    TokenStream::from(expanded)
}
