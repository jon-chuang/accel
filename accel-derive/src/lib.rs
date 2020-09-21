#![recursion_limit = "128"]

//! Get compiled PTX as `String`
//! ----------------------------
//!
//! The proc-macro `#[kernel]` creates a submodule `add::` in addition to a function `add`.
//! Kernel Rust code is compiled into PTX string using rustc's `nvptx64-nvidia-cuda` toolchain.
//! Generated PTX string is embedded into proc-macro output as `{kernel_name}::PTX_STR`.
//!
//! ```
//! use accel_derive::kernel;
//!
//! #[kernel]
//! unsafe fn add(a: *const f64, b: *const f64, c: *mut f64, n: usize) {
//!     let i = accel_core::index();
//!     if (i as usize) < n {
//!         *c.offset(i) = *a.offset(i) + *b.offset(i);
//!     }
//! }
//!
//! // PTX assembler code is embedded as `add::PTX_STR`
//! println!("{}", add::PTX_STR);
//! ```

mod builder;
mod contexted;
mod host;
mod launchable;
mod parser;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn kernel_mod(_attr: TokenStream, func: TokenStream) -> TokenStream {
    let module: syn::ItemMod = syn::parse(func).expect("Not a function");
    let (ptx_str, func, content) = builder::compile_tokens_mod(&module).expect("Failed to compile to PTX");
    let res = host::mod2caller(&ptx_str, &func, content).into();
    res
}

// We just need an attribute to match against
#[proc_macro_attribute]
pub fn kernel_func(_attr: TokenStream, func: TokenStream) -> TokenStream {
    func
}

#[proc_macro_attribute]
pub fn kernel(_attr: TokenStream, func: TokenStream) -> TokenStream {
    let func: syn::ItemFn = syn::parse(func).expect("Not a function");
    let ptx_str = builder::compile_tokens(&func).expect("Failed to compile to PTX");
    host::func2caller(&ptx_str, &func).into()
}

#[proc_macro_derive(Contexted)]
pub fn contexted(input: TokenStream) -> TokenStream {
    contexted::contexted(syn::parse(input).unwrap()).into()
}

#[proc_macro]
pub fn define_launchable(item: TokenStream) -> TokenStream {
    launchable::generate(item.into()).into()
}
