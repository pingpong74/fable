use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, parse_macro_input};

#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    let name_str = name.to_string().to_uppercase();
    let id_static_name = quote::format_ident!("{}_ID_INTERNAL", name_str);
    let info_static_name = quote::format_ident!("{}_INFO_INTERNAL", name_str);

    let expanded = quote! {
        #input

        static #id_static_name: ComponentId = ComponentId::invalid();

        #[linkme::distributed_slice(COMPONENTS_POOL)]
        static #info_static_name: ComponentInfo = ComponentInfo::of::<#name>(&#id_static_name);

        impl Component for #name {
            const INFO: &'static ComponentInfo = &#info_static_name;
            const ID: &'static ComponentId = &#id_static_name;
        }
    };

    TokenStream::from(expanded)
}
