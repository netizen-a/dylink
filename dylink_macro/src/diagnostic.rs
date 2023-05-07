// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

// This emits warnings for attributes where applicable
#[cfg(feature = "warnings")]
pub(crate) fn foreign_mod_diag(foreign_mod: &syn::ItemForeignMod) {
    use syn::spanned::Spanned;
    let mut doc_spans = Vec::new();
    for mod_attr in foreign_mod.attrs.iter() {
        if mod_attr.path.is_ident("doc") {
            doc_spans.push(mod_attr.span());
        }
        if mod_attr.path.is_ident("repr") {
            // TODO: refine diagnostic to look similar to official error
            proc_macro::Diagnostic::spanned(
                mod_attr.span().unwrap(),
                proc_macro::Level::Error,
                "attribute should be applied to a struct, enum, or union",
            )
            .emit();
        }
    }
    if !doc_spans.is_empty() {
        let mut span = doc_spans.remove(0);
        for item in doc_spans.into_iter() {
            span = span.join(item).unwrap();
        }
        proc_macro::Diagnostic::spanned(
            span.unwrap(),
            proc_macro::Level::Warning,
            "unused doc comment",
        )
        .help("use `//` for a plain comment")
        .emit();
    }
}
