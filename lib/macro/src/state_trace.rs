use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

pub fn trace_state_access(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let attrs = &input_fn.attrs;
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let block = &input_fn.block;

    if sig.ident != "current_data_point" {
        panic!("trace_state can only be applied to `current_data_point` methods");
    }

    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            use tracing::Instrument;

            let span = tracing::trace_span!(
                "DataPointAccess::current_data_point",
                otel.name = tracing::field::Empty,
                state_type = self.ext_id().type_name(),
                state_name = self.ext_id().variant_name(),
                dp.value = tracing::field::Empty,
                dp.timestamp = tracing::field::Empty,
                dp.elapsed = tracing::field::Empty,
            );

            let result = async move #block
                .instrument(span.clone())
                .await;

            if let Ok(ref dp) = result {
                span.record("otel.name", format!("{} - {}", self.ext_id(), dp.value));
                span.record("dp.value", dp.value.to_string());
                span.record("dp.timestamp", dp.timestamp.to_iso_string());
                span.record("dp.elapsed", dp.timestamp.elapsed().to_iso_string());
            } else {
                span.record("otel.name", format!("{} - error", self.ext_id()));
            }

            result
        }
    };

    expanded.into()
}
