extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn trisult(_attr: TokenStream, _input: TokenStream) -> TokenStream {
    todo!()
}
