extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;

#[proc_macro_derive(BachModuleStdTests)]
pub fn _bach_std_test_derive(input: TokenStream) -> TokenStream {
    let syn_tree: syn::DeriveInput = syn::parse(input).unwrap();
    let st_name = syn_tree.ident;
    let name_test_ident = syn::Ident::new(
        &format!("bach_module_std_name_test_for_{}", st_name).to_lowercase(),
        Span::call_site(),
    );
    let init_test_ident = syn::Ident::new(
        &format!("bach_module_std_init_test_for_{}", st_name).to_lowercase(),
        Span::call_site(),
    );
    let accept_test_ident = syn::Ident::new(
        &format!("bach_module_std_accept_test_for_{}", st_name).to_lowercase(),
        Span::call_site(),
    );
    let inlet_test_ident = syn::Ident::new(
        &format!("bach_module_std_inlet_test_for_{}", st_name).to_lowercase(),
        Span::call_site(),
    );
    let destroy_test_ident = syn::Ident::new(
        &format!("bach_module_std_destroy_test_for_{}", st_name).to_lowercase(),
        Span::call_site(),
    );
    let outlet_test_ident = syn::Ident::new(
        &format!("bach_module_std_outlet_test_for_{}", st_name).to_lowercase(),
        Span::call_site(),
    );
    let spawn_test_ident = syn::Ident::new(
        &format!("bach_module_std_spawn_test_for_{}", st_name).to_lowercase(),
        Span::call_site(),
    );
    let stdtest_modname_ident = syn::Ident::new(
        &format!("{}stdtests", st_name).to_lowercase(),
        Span::call_site(),
    );

    let conf_examples_dir = format!("./modules/{}/", st_name).to_lowercase();

    let tests = quote! {
        #[cfg(test)]
        mod #stdtest_modname_ident {
            use crate::#st_name;
            use bach_module::Module;
            use bach_bus::packet::*;
            use std::io;
            use std::fs::{self, DirEntry};
            use std::path::Path;
            use std::time::{Instant, Duration};
            use std::thread;

            static NONBLOCK_TIMEOUT: u64 = 5;

            fn list_configs() -> std::io::Result<Vec<String>> {
                let mut ret = Vec::new();
                let dir = Path::new(#conf_examples_dir);
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let p = entry.path();
                    if let Some(s) = p.to_str() {
                        if p.is_file() && s.contains("example.config") {
                            ret.push(s.to_string());
                        }
                    }
                }
                Ok(ret)
            }

            #[test]
            fn #name_test_ident () {
                let module = #st_name::new(None);
                assert!(!module.name().is_empty());
                if let Ok(configs) = list_configs() {
                    for c in configs {
                        let module = #st_name::new(Some(c));
                        assert!(!module.name().is_empty());
                    }
                }
            }

            #[test]
            fn #init_test_ident () {
                let module = #st_name::new(None);
                let result = module.init();
                assert!(result.is_ok());
                if let Ok(configs) = list_configs() {
                    for c in configs {
                        let module = #st_name::new(Some(c));
                        let result = module.init();
                        assert!(result.is_ok());
                    }
                }
            }

            #[test]
            fn #accept_test_ident () {
                let module = #st_name::new(None);
                let termp = Packet::Terminate;
                let start = Instant::now();
                assert!(module.accept(termp));
                assert!(start.elapsed().le(&Duration::from_millis(NONBLOCK_TIMEOUT)));
                if let Ok(configs) = list_configs() {
                    for c in configs {
                        let module = #st_name::new(Some(c));
                        let start = Instant::now();
                        assert!(module.accept(termp));
                        assert!(start.elapsed().le(&Duration::from_millis(NONBLOCK_TIMEOUT)));
                    }
                }
            }

            #[test]
            fn #inlet_test_ident () {
                let module = #st_name::new(None);
                let start = Instant::now();
                let termp = Packet::Terminate;
                module.inlet(termp);
                assert!(start.elapsed().le(&Duration::from_millis(NONBLOCK_TIMEOUT)));
                if let Ok(configs) = list_configs() {
                    for c in configs {
                        let module = #st_name::new(Some(c));
                        let start = Instant::now();
                        module.inlet(termp);
                        assert!(start.elapsed().le(&Duration::from_millis(NONBLOCK_TIMEOUT)));
                    }
                }
            }

            #[test]
            fn #destroy_test_ident () {
                let module = #st_name::new(None);
                let res = module.destroy();
                assert!(res.is_ok());
                if let Ok(configs) = list_configs() {
                    for c in configs {
                        let module = #st_name::new(Some(c));
                        let res = module.destroy();
                        assert!(res.is_ok());
                    }
                }
            }

            #[test]
            fn #outlet_test_ident () {
                let module = #st_name::new(None);
                let start = Instant::now();
                module.outlet();
                assert!(start.elapsed().le(&Duration::from_millis(NONBLOCK_TIMEOUT)));
                if let Ok(configs) = list_configs() {
                    for c in configs {
                        let module = #st_name::new(Some(c));
                        let start = Instant::now();
                        module.outlet();
                        assert!(start.elapsed().le(&Duration::from_millis(NONBLOCK_TIMEOUT)));
                    }
                }
            }

            #[test]
            fn #spawn_test_ident () {
                let test_spawn = |opt: Option<String>| {
                    let module = #st_name::new(opt);
                    module.init().unwrap();
                    let start = Instant::now();
                    let joinhandle = module.spawn();
                    let jp = thread::spawn(|| {
                        thread::sleep(Duration::from_secs(30));
                        panic!();
                    });
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(500));
                        let termp = Packet::Terminate;
                        module.inlet(termp);
                    });
                    let res = joinhandle.join().unwrap();
                    assert!(res.is_ok());
                };

                test_spawn(None);
                if let Ok(configs) = list_configs() {
                    for c in configs {
                        test_spawn(Some(c));
                    }
                }
            }
        }
    };

    tests.into()
}
