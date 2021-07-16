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

    let input_test_ident = syn::Ident::new(
        &format!("bach_module_std_input_test_for_{}", st_name).to_lowercase(),
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

    let output_test_ident = syn::Ident::new(
        &format!("bach_module_std_output_test_for_{}", st_name).to_lowercase(),
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

    let fire_test_ident = syn::Ident::new(
        &format!("bach_module_std_fire_test_for_{}", st_name).to_lowercase(),
        Span::call_site(),
    );


    let tests = quote! {
        #[cfg(test)]
        mod #stdtest_modname_ident {
            use crate::#st_name;
            use bach_module::{Module, self};
            use bach_bus::packet::*;
            use std::io;
            use std::fs::{self, DirEntry};
            use std::path::{Path, PathBuf};
            use std::time::{Instant, Duration};
            use std::thread;
            use std::sync::{
                Arc, Mutex,
                atomic::{
                    AtomicU8, AtomicBool, Ordering,
                }
            };
            use std::cell::RefCell;
            use std::process::Command;

            static NONBLOCK_TIMEOUT: u64 = 5;

            fn list_configs() -> std::io::Result<Vec<String>> {
                let mut ret = Vec::new();
                let dir = Path::new("./");
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let p = entry.path();
                    if let Some(s) = p.to_str() {
                        if p.is_file() && s.contains("example.config") {
                            ret.push(s.to_string());
                        }
                    }
                }
                ret.sort();
                Ok(ret)
            }

            fn list_post_checks() -> std::io::Result<Vec<String>> {
                let mut ret = Vec::new();
                let dir = Path::new("./");
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let p = entry.path();
                    if let Some(s) = p.to_str() {
                        if p.is_file() && s.contains("post-check") && s.contains(".py") {
                            ret.push(s.to_string());
                        }
                    }
                }
                ret.sort();
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
            fn #input_test_ident () {
                let module = #st_name::new(None);
                let start = Instant::now();
                let termp = Packet::Terminate;
                module.input(termp);
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
                module.outlet(Packet::new_alive(&module.name()));
                let out = module.output();
                assert!(out.is_some());
                assert_eq!(out.unwrap(), Packet::new_alive(&module.name()));
            }

            #[test]
            fn #output_test_ident () {
                let module = #st_name::new(None);
                let start = Instant::now();
                module.output();
                assert!(start.elapsed().le(&Duration::from_millis(NONBLOCK_TIMEOUT)));
                if let Ok(configs) = list_configs() {
                    for c in configs {
                        let module = #st_name::new(Some(c));
                        let start = Instant::now();
                        module.output();
                        assert!(start.elapsed().le(&Duration::from_millis(NONBLOCK_TIMEOUT)));
                    }
                }
            }

            #[test]
            fn #fire_test_ident () {
                let test_fire = |opt: Option<String>| {
                    let optcopy = match &opt {
                        Some(s) => Some(s.to_string()),
                        None => None,
                    };
                    let optcopy2 = match &opt {
                        Some(s) => Some(s.to_string()),
                        None => None,
                    };

                    let module = #st_name::new(opt);
                    let message_stack: Arc<Mutex<RefCell<Vec<Packet>>>>
                        = Arc::new(Mutex::new(RefCell::new(Vec::new())));
                    let run_control: Arc<AtomicU8> = Arc::new(AtomicU8::new(bach_module::RUN_FIRE));
                    let conf_arc: Arc<Mutex<RefCell<Option<PathBuf>>>> = match optcopy {
                        Some(s) => Arc::new(Mutex::new(RefCell::new(Some(PathBuf::from(s))))),
                        None => Arc::new(Mutex::new(RefCell::new(None))),
                    };
                    let name_arc = Arc::new(Mutex::new(RefCell::new(module.name())));

                    let main_method = module.fire();
                    let result = main_method(&message_stack,&run_control, &conf_arc, &name_arc);
                    let controlafter = run_control.load(Ordering::SeqCst);
                    assert!(controlafter == bach_module::RUN_IDLE || controlafter == bach_module::RUN_EARLY_TERM);
                    assert!(result.is_ok());
                };

                test_fire(None);

                if let Ok(configs) = list_configs() {
                    for c in configs {
                        println!("Using configuration {}", c);
                        test_fire(Some(c));
                    }
                }
                if let Ok(v) = list_post_checks() {
                    for check in v {
                        println!("Doing Check {}", check);
                        let status = Command::new("python3")
                            .arg(&check)
                            .status()
                            .expect(&format!("Failed to execute post test {}", check));
                        assert!(status.success());
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
                    thread::sleep(Duration::from_millis(500));
                    let termp = Packet::Terminate;
                    module.input(termp);

                    let mut received_alive = false;

                    for _ in 0..10 {
                        if let Some(Packet::Alive(_)) = module.output() {
                            received_alive = true;
                        }
                        thread::sleep(Duration::from_millis(250));
                    }
                    assert!(received_alive);
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
