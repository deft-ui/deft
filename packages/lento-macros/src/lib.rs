use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn};

#[proc_macro_attribute]
pub fn js_func(_attr: TokenStream, func: TokenStream) -> TokenStream {
    let func = parse_macro_input!(func as ItemFn);
    let vis = func.vis;
    let func_name = &func.sig.ident;
    let func_name_str = func_name.to_string();
    let func_inputs = func.sig.inputs;
    let func_block = func.block;
    let params: Vec<_> = func_inputs.iter().map(|i| {
        match i {
            FnArg::Receiver(_) => unreachable!(),
            FnArg::Typed(ref val) => {
                &val.ty
            }
        }
    }).collect();
    let mut param_list = Vec::new();
    let mut idx = 0usize;
    for p in params {
        param_list.push(quote! {
            #p::from_js_value(args.get(#idx).unwrap().clone())?
        });
        idx += 1;
    }

    let return_type = func.sig.output;

    let expanded = quote! {

        #[doc(hidden)]
        #[allow(nonstandard_style)]
        #vis struct #func_name  {}

        impl #func_name {

            fn #func_name(#func_inputs) #return_type #func_block

            pub fn new() -> Self {
                Self {}
            }

        }

        impl lento::js::JsFunc for #func_name {
            fn name(&self) -> &str {
                #func_name_str
            }

            fn args_count(&self) -> usize {
                #idx
            }

            fn call(&self, js_context: &mut lento::mrc::Mrc<lento::js::JsContext>, args: Vec<lento::js::JsValue>) -> Result<lento::js::JsValue, lento::js::JsCallError> {
                use lento::js::FromJsValue;
                use lento::js::ToJsValue;
                use lento::js::ToJsCallResult;

                let r = Self::#func_name( #(#param_list, )* );
                r.to_js_call_result()
            }
        }

    };
    expanded.into()
}