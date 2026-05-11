extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn trisult(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(input as ItemFn);
    let original_block = &func.block;
    let expanded_block = quote! {
        {
            let mut __trisult_diags = ::trisult::Diagnoses::new(::trisult::AccumulatorKind::All);
            let mut __has_errors = false;

            macro_rules! warn {
                ($warn:expr, $ctx:expr) => {
                    __trisult_diags.push_naive(::trisult::Contextual::new(
                        $ctx,
                        ::trisult::Diagnosis::Warning($warn)
                    ));
                };
            }

            macro_rules! error {
                ($err:expr, $ctx:expr) => {
                    __has_errors = true;
                    __trisult_diags.push_naive(::trisult::Contextual::new(
                        $ctx,
                        ::trisult::Diagnosis::Error($err)
                    ));
                };
            }

            macro_rules! tri {
                ($expr:expr) => {
                    match $expr {
                        ::trisult::Trisult::Ok(::trisult::Diagnosed(val, diags)) => {
                            __trisult_diags.append_naive(diags.map(::trisult::Diagnosis::Warning));
                            Some(val)
                        }
                        ::trisult::Trisult::Err(diags) => {
                            __has_errors = true;
                            __trisult_diags.append(diags);
                            None
                        }
                    }
                };
            }

            let mut __trisult_body = || {
                #original_block
            };
            let __trisult_res = __trisult_body();

            if __has_errors || __trisult_res.is_none() {
                ::trisult::Trisult::Err(__trisult_diags)
            } else {
                let __warnings = __trisult_diags.unwrap_as_warnings();
                ::trisult::Trisult::Ok(::trisult::Diagnosed(__trisult_res.unwrap(), __warnings))
            }
        }
    };

    *func.block = syn::parse2(expanded_block).expect("Failed to parse expanded block");

    quote! { #func }.into()
}
