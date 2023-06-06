// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use proc_macro2::Span;
use syn::punctuated::Punctuated;
use syn::{spanned::Spanned, *};

pub struct AttrData {
	pub link_ty: LinkType,
	pub library: Option<syn::Path>,
	pub link_name: Option<(String, Span)>,
}

#[derive(PartialEq)]
pub enum LinkType {
	Vulkan,
	// note: dylink_macro must use an owned string instead of `&'static [u8]` since it's reading from the source code.
	General,
}

impl TryFrom<Punctuated<Expr, Token!(,)>> for AttrData {
	type Error = syn::Error;
	fn try_from(value: Punctuated<Expr, Token!(,)>) -> Result<Self> {
		let mut maybe_link_ty: Option<LinkType> = None;
		let mut library: Option<syn::Path> = None;
		let mut link_name: Option<(String, Span)> = None;
		let mut errors = vec![];
		const EXPECTED_KW: &str = "Expected `vulkan`, `link_name`, or `library`.";

		for expr in value.iter() {
			match expr {
				// Branch for syntax: #[dylink(vulkan)]
				Expr::Path(ExprPath { path, .. }) => {
					if path.is_ident("vulkan") {
						if maybe_link_ty.is_none() {
							maybe_link_ty = Some(LinkType::Vulkan);
						} else {
							errors.push(Error::new(path.span(), "Library already defined."));
						}
					} else {
						errors.push(Error::new(path.span(), EXPECTED_KW));
					}
				}
				Expr::Assign(assign) => {
					let (assign_left, assign_right) = (assign.left.as_ref(), assign.right.as_ref());

					let Expr::Path(ExprPath { path, .. }) = assign_left else {
						unreachable!("internal error when parsing Expr::Assign");
					};
					if path.is_ident("library") {
						// Branch for syntax: #[dylink(library = <path>)]
						match assign_right {
							Expr::Path(ExprPath{path, .. }) => {
								if library.is_none() {
									maybe_link_ty = Some(LinkType::General);
									library = Some(path.clone());
								} else {
									errors.push(Error::new(
										assign.span(),
										"library is already defined",
									));
								}
							}
							right => errors.push(Error::new(right.span(), "Expected identifier.")),
						}
					} else if path.is_ident("link_name") {
						// Branch for syntax: #[dylink(link_name = <string>)]
						match assign_right {
							Expr::Lit(ExprLit {
								lit: Lit::Str(val), ..
							}) => {
								if link_name.is_none() {
									link_name = Some((val.value(), assign.span()));
								} else {
									errors.push(Error::new(
										assign.span(),
										"linker is already defined",
									));
								}
							}
							right => errors.push(Error::new(right.span(), "Expected string.")),
						}
					} else {
						errors.push(Error::new(assign_left.span(), EXPECTED_KW));
					}
				}
				
				// Branch for everything else.
				expr => errors.push(Error::new(expr.span(), EXPECTED_KW)),
			}
		}

		if maybe_link_ty.is_none() {
			errors.push(Error::new(
				value.span(),
				"No library detected. Suggested: use `vulkan` or `library = <path>`.",
			));
		}

		// if there are any errors this will immediately combine and return early.
		if !errors.is_empty() {
			if let Some(mut main_err) = errors.pop() {
				for err in errors {
					main_err.combine(err);
				}
				Err(main_err)
			} else {
				// argument list was empty. this is a problem
				Err(Error::new(value.span(), EXPECTED_KW))
			}
		} else {
			Ok(Self {
				link_ty: maybe_link_ty.unwrap(),
				library,
				link_name,
			})
		}
	}
}
