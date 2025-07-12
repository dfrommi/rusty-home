use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

pub fn mockable_state(item: TokenStream) -> TokenStream {
    //let args = parse_macro_input!(attr as AttributeArgs);
    let input_fn = parse_macro_input!(item as ItemFn);

    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let block = &input_fn.block;

    let expanded = if sig.ident == "current_data_point" {
        quote! {
            #vis #sig {
                #[cfg(test)]
                if let Some(dp) = api.get_fixed_current_dp(self.clone()) {
                    return Ok(dp);
                }

                #block
            }
        }
    } else if sig.ident == "series" {
        quote! {
            #vis #sig {
                #[cfg(test)]
                if let Some(df) = api.get_fixed_ts(self.clone()) {
                    return crate::core::timeseries::TimeSeries::new(self.clone(), &df, range);
                }

                #block
            }
        }
    } else {
        panic!("Method with name {} is not allowed", sig.ident);
    };

    expanded.into()
}
