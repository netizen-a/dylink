// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![cfg_attr(feature = "warnings", feature(proc_macro_diagnostic))]

mod attr_data;

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::{parse::Parser, punctuated::Punctuated, spanned::Spanned, Expr, Token};

use attr_data::*;
use syn::ForeignItem;

#[proc_macro_attribute]
pub fn dylink(args: TokenStream1, input: TokenStream1) -> TokenStream1 {
	let punct = Parser::parse2(
		Punctuated::<Expr, Token!(,)>::parse_separated_nonempty,
		args.into(),
	)
	.expect("failed to parse");

	match AttrData::try_from(punct) {
		Ok(attr_data) => {
			if let Ok(foreign_mod) = syn::parse2::<syn::ItemForeignMod>(input.clone().into()) {
				if let Some((_, span)) = attr_data.link_name {
					return syn::Error::new(
						span,
						"`link_name` should be applied to a foreign function",
					)
					.to_compile_error()
					.into();
				}

				let abi = &foreign_mod.abi;
				foreign_mod
					.items
					.iter()
					.map(|item| match item {
						ForeignItem::Fn(fn_item) => parse_fn::<true>(Some(abi), fn_item, &attr_data),
						other => quote!(#abi {#other}),
					})
					.collect::<TokenStream2>()
					.into()
			} else if let Ok(foreign_fn) = syn::parse2::<syn::ForeignItemFn>(input.into()) {
				parse_fn::<false>(foreign_fn.sig.abi.as_ref(), &foreign_fn, &attr_data).into()
			} else {
				panic!("failed to parse");
			}
		}
		Err(e) => syn::Error::into_compile_error(e).into(),
	}
}

fn parse_fn<const IS_MOD_ITEM: bool>(
	abi: Option<&syn::Abi>,
	fn_item: &syn::ForeignItemFn,
	attr_data: &AttrData,
) -> TokenStream2 {
	let abi = abi.to_token_stream();
	let fn_name = fn_item.sig.ident.to_token_stream();	
	let vis = fn_item.vis.to_token_stream();
	let output = fn_item.sig.output.to_token_stream();
	let strip = attr_data.strip;
	let link_ty = &attr_data.link_ty;
	let linker = &attr_data.linker;

	let fn_attrs: Vec<TokenStream2> = fn_item
		.attrs
		.iter()
		.map(syn::Attribute::to_token_stream)
		.collect();

	if let syn::ReturnType::Type(_, ret_type) = &fn_item.sig.output {
		if let syn::Type::Path(syn::TypePath { path, .. }) = ret_type.as_ref() {
			if path.is_ident("Self") {
				return syn::Error::new(
					path.span(),
					"`Self` cannot be inferred. Try using an explicit type instead",
				)
				.to_compile_error();
			}
		}
	}

	let mut param_list = Vec::new();
	let mut param_ty_list = Vec::new();
	let mut internal_param_ty_list = Vec::new();
	let mut internal_param_list = Vec::new();
	let params_default = fn_item.sig.inputs.to_token_stream();
	for (i, arg) in fn_item.sig.inputs.iter().enumerate() {
		match arg {
			syn::FnArg::Typed(pat_type) => {
				if let syn::Type::Path(syn::TypePath { path, .. }) = pat_type.ty.as_ref() {
					if path.is_ident("Self") {
						return syn::Error::new(
							path.span(),
							"`Self` cannot be inferred. Try using an explicit type instead",
						)
						.to_compile_error();
					}
				}
				let ty = pat_type.ty.to_token_stream();
				let param_name = match pat_type.pat.as_ref() {
					syn::Pat::Wild(_) => format!("p{i}").parse::<TokenStream2>().unwrap(),
					syn::Pat::Ident(pat_id) => pat_id.ident.to_token_stream(),
					_ => unreachable!(),
				};
				param_list.push(param_name.clone());
				internal_param_list.push(param_name.clone());
				param_ty_list.push(quote!(#param_name : #ty));
				internal_param_ty_list.push(quote!(#param_name : #ty));
			}
			syn::FnArg::Receiver(rec) => {
				if IS_MOD_ITEM || strip {
					// TODO: fix error message
					return syn::Error::new(
						rec.span(),
						"`self` arguments are unsupported in this context",
					)
					.into_compile_error();
				} else {
					if let syn::Type::Path(syn::TypePath { path, .. }) = rec.ty.as_ref() {
						if path.is_ident("Self") {
							return syn::Error::new(
								path.span(),
								"type of `self` cannot be inferred. Try using an explicit type instead",
							)
							.to_compile_error();
						}
					}
					let ty = rec.ty.to_token_stream();
					let param_name = format!("p{i}").parse::<TokenStream2>().unwrap();
					param_list.push(quote! {self});
					internal_param_list.push(param_name.clone());
					param_ty_list.push(quote!(self : #ty));
					internal_param_ty_list.push(quote!(#param_name : #ty));
				}
			}
		}
	}

	// this is sure to obfuscate things, but this is needed here because `strip` screws with call context.
	let caller_name = if strip {
		quote! {function}
	} else {
		quote! {DYN_FUNC}
	};

	let std_transmute = quote! {std::mem::transmute};

	let call_dyn_func = if *link_ty == LinkType::Vulkan {
		match fn_name.to_string().as_str() {
			"vkCreateInstance" => {
				if strip {
					return syn::Error::new(
						fn_item.span(),
						"`vkCreateInstance` is incompatible with `strip=true`",
					)
					.to_compile_error();
				}
				let inst_param = &param_list[2];
				quote! {
					let result = #caller_name(#(#param_list),*);
					unsafe {
						dylink::Global.insert_instance(
							*#std_transmute::<_, *mut dylink::vk::Instance>(#inst_param)
						);
					}
					result
				}
			}
			"vkDestroyInstance" => {
				if strip {
					return syn::Error::new(
						fn_item.span(),
						"`vkDestroyInstance` is incompatible with `strip=true`",
					)
					.to_compile_error();
				}
				let inst_param = &param_list[0];
				quote! {
					let result = #caller_name(#(#param_list),*);
					unsafe {
						dylink::Global.remove_instance(&#std_transmute::<_, dylink::vk::Instance>(#inst_param));
					}
					result
				}
			}
			"vkCreateDevice" => {
				if strip {
					return syn::Error::new(
						fn_item.span(),
						"`vkCreateDevice` is incompatible with `strip=true`",
					)
					.to_compile_error();
				}
				let inst_param = &param_list[3];
				quote! {
					let result = #caller_name(#(#param_list),*);
					unsafe {
						dylink::Global.insert_device(*#std_transmute::<_, *mut dylink::vk::Device>(#inst_param));
					}
					result
				}
			}
			"vkDestroyDevice" => {
				if strip {
					return syn::Error::new(
						fn_item.span(),
						"`vkDestroyDevice` is incompatible with `strip=true`",
					)
					.to_compile_error();
				}
				let inst_param = &param_list[0];
				quote! {
					let result = #caller_name(#(#param_list),*);
					unsafe {
						dylink::Global.remove_device(&#std_transmute::<_, dylink::vk::Device>(#inst_param));
					}
					result
				}
			}
			_ => quote!(#caller_name(#(#param_list),*)),
		}
	} else {
		quote!(#caller_name(#(#param_list),*))
	};

	let try_link_call = match linker {
		None => quote! {
			try_link
		},
		Some(linker_name) => quote! {
			try_link_with::<#linker_name>
		},
	};

	let lint;
	let link_name = match &attr_data.link_name {
		Some((name, _)) => {
			lint = TokenStream2::default();
			name.clone()
		}
		None => {
			lint = if strip {
				quote! {#[allow(non_upper_case_globals)]}
			} else {
				quote! {#[allow(non_snake_case)]}
			};
			fn_name.to_string()
		}
	};
	//println!("{fn_name}:{abi}");
	
	// According to "The Rustonomicon" foreign functions are assumed unsafe,
	// so functions are implicitly prepended with `unsafe`
	if strip {
		quote! {
			#(#fn_attrs)*
			#lint
			#vis static #fn_name
			: dylink::LazyFn<'static, unsafe #abi fn (#params_default) #output>
			= dylink::LazyFn::new(
				{
					type InstFnPtr = unsafe #abi fn (#params_default) #output;
					unsafe #abi fn initial_fn (#(#param_ty_list),*) #output {
						use std::ffi::CStr;
						match #fn_name.#try_link_call() {
							Ok(function) => {#call_dyn_func},
							Err(err) => panic!("{}", err),
						}
					}
					const DYN_FUNC_REF: &'static InstFnPtr = &(initial_fn as InstFnPtr);
					DYN_FUNC_REF
				},
				unsafe {std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(#link_name, '\0').as_bytes())},
				dylink::#link_ty
			);
		}
	} else {
		quote! {
			#(#fn_attrs)*
			#lint
			#[inline]
			#vis unsafe #abi fn #fn_name (#(#param_ty_list),*) #output {
				// InstFnPtr: instance function pointer type
				type InstFnPtr = #abi fn (#(#internal_param_ty_list),*) #output;
				#abi fn initial_fn (#(#internal_param_ty_list),*) #output {
					use std::ffi::CStr;
					match DYN_FUNC.#try_link_call() {
						Ok(function) => {function(#(#internal_param_list),*)},
						Err(err) => panic!("{}", err),
					}
				}
				const DYN_FUNC_REF: &'static InstFnPtr = &(initial_fn as InstFnPtr);
				static DYN_FUNC
				: dylink::LazyFn<'static, InstFnPtr>
				= dylink::LazyFn::new(
					DYN_FUNC_REF,
					unsafe {std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(#link_name, '\0').as_bytes())},
					dylink::#link_ty
				);

				#call_dyn_func
			}
		}
	}
}
