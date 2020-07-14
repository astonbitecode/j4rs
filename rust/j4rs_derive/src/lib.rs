// Copyright 2020 astonbitecode
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
extern crate proc_macro;
extern crate proc_macro2;

use proc_macro::TokenStream;

use proc_macro2::{Ident, Span};
use syn::{AttributeArgs, Expr, FnArg, ItemFn, Lit, NestedMeta, parse_macro_input, ReturnType};

use quote::quote;

#[proc_macro_attribute]
pub fn call_from_java(macro_args: TokenStream, user_function: TokenStream) -> TokenStream {
    let cloned_user_function = user_function.clone();
    let macro_args = parse_macro_input!(macro_args as AttributeArgs);
    let user_function = parse_macro_input!(user_function as ItemFn);
    let mut generated = impl_call_from_java_macro(&user_function, macro_args);

    generated.extend(cloned_user_function.into_iter());
    generated
}

fn impl_call_from_java_macro(user_function: &ItemFn, macro_args: AttributeArgs) -> TokenStream {
    let mut macro_args = macro_args;
    // Retrieve the Ident for the jni function
    let jni_ident_string = match macro_args.pop().expect("No args found in call_from_java. Usage: #[call_from_java(\"full.class.name\")]") {
        NestedMeta::Lit(Lit::Str(litstr)) => {
            format!("Java_{}", litstr.value().replace(".", "_"))
        }
        _ => panic!("No valid args found in call_from_java. Usage: #[call_from_java(\"full.class.name\")]"),
    };
    let ref jni_ident = Ident::new(jni_ident_string.as_ref(), Span::call_site());
    // Retrieve the user function Ident, input arguments and return output
    // Ident
    let user_function_signature = &user_function.sig;
    let user_function_name = &user_function_signature.ident;
    // Arguments
    let user_function_args = &user_function_signature.inputs;
    // The argument names as defined by the user
    let user_function_arg_names: Vec<String> = user_function_args.iter()
        .map(|arg| {
            let a = arg.clone();
            let q = quote!(#a).to_string();
            let v: Vec<&str> = q.split(' ').collect();
            v.get(0).expect(&format!("Could not locate the argument name for: {}", q)).to_string()
        })
        .collect();
    // The arguments of the jni function
    let jni_function_args: Vec<FnArg> = user_function_arg_names.iter()
        .map(|arg| {
            let a: FnArg = syn::parse_str(&format!("{}: jobject", arg)).unwrap();
            a
        })
        .collect();
    // The jni function return type
    let ref jni_function_output = match &user_function_signature.output {
        ReturnType::Default => ReturnType::Default,
        _ => {
            let ret_type: ReturnType = syn::parse_str("-> jobject").unwrap();
            ret_type
        }
    };
    // The jni return value. This may be void or jobject
    let return_value = match &user_function_signature.output {
        ReturnType::Default => {
            let ret_value: Expr = syn::parse_str("()").unwrap();
            ret_value
        }
        _ => {
            let ret_value: Expr = syn::parse_str(
                r#"match instance_to_return {
                    Ok(i) => {
                        i.java_object()
                        // i.as_java_ptr_with_local_ref(jni_env).unwrap()
                    },
                    Err(error) => {
                        let message = format!("{}", error);
                        let _ = jvm.throw_invocation_exception(&message);
                        ptr::null_mut()
                    },
                }"#).unwrap();
            ret_value
        }
    };
    let instance_args_to_pass_to_user_function: Vec<Expr> = user_function_arg_names.iter()
        .map(|jobj_arg_name| {
            let expression: Expr = syn::parse_str(&format!("Instance::from_jobject_with_global_ref({}).expect(\"Could not create Instance from jobject\")", jobj_arg_name)).unwrap();
            expression
        })
        .collect();

    let gen = quote! {
        #[no_mangle]
        pub fn #jni_ident(jni_env: *mut JNIEnv, _class: *const c_void, #(#jni_function_args),*) #jni_function_output {
            match Jvm::try_from(jni_env) {
                Ok(mut jvm) => {
                    jvm.detach_thread_on_drop(false);
                    // println!("Called {}. Calling now  {}", stringify!(#jni_ident), stringify!(#user_function_name));
                    let instance_to_return = #user_function_name(#(#instance_args_to_pass_to_user_function),*);
                    #return_value
                },
                Err(error) => {
                    let message = format!("Could not attach to the JVM thread: {}", error);
                    println!("{}", message);
                    panic!(message);
                },
            }
        }
    };
    gen.into()
}
