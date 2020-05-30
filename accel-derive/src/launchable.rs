use proc_macro2::{Span, TokenStream};
use quote::quote;

pub fn generate(item: TokenStream) -> TokenStream {
    let literal: syn::LitInt = syn::parse2(item).unwrap();
    let n: usize = literal.base10_parse().unwrap();
    (0..=n)
        .into_iter()
        .map(|i| {
            let name = syn::Ident::new(&format!("Launchable{}", i), Span::call_site());
            let targets: Vec<syn::Ident> = (1..=i)
                .into_iter()
                .map(|k| syn::Ident::new(&format!("Target{}", k), Span::call_site()))
                .collect();
            let args_value: Vec<syn::Ident> = (1..=i)
                .into_iter()
                .map(|k| syn::Ident::new(&format!("arg{}", k), Span::call_site()))
                .collect();
            let args_types: Vec<syn::Ident> = (1..=i)
                .into_iter()
                .map(|k| syn::Ident::new(&format!("Arg{}", k), Span::call_site()))
                .collect();
            quote! {
                pub trait #name <'arg> {
                    #(
                        type #targets;
                    )*

                    fn get_kernel(&self) -> Result<Kernel>;

                    fn launch<#(#args_types),*>(
                        &self,
                        grid: impl Into<Grid>,
                        block: impl Into<Block>,
                        (#(#args_value,)*): (#(#args_types,)*),
                    ) -> Result<()>
                    where
                        #(
                            #args_types: DeviceSend<Target = Self::#targets>
                        ),*
                    {
                        let grid = grid.into();
                        let block = block.into();
                        let kernel = self.get_kernel()?;
                        let mut args = [#(#args_value.as_kernel_parameter()),*];
                        unsafe {
                            contexted_call!(
                                &kernel,
                                cuLaunchKernel,
                                kernel.func,
                                grid.x,
                                grid.y,
                                grid.z,
                                block.x,
                                block.y,
                                block.z,
                                0,          /* FIXME: no shared memory */
                                null_mut(), /* use default stream */
                                args.as_mut_ptr(),
                                null_mut() /* no extra */
                            )?;
                        }
                        kernel.sync()?;
                        Ok(())
                    }
                }
            }
        })
        .collect()
}
