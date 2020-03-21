use quote::ToTokens;
use syn::Error;
use std::fmt::Display;

pub struct Ctxt {
    errors: Vec<Error>,
}

impl Ctxt {
    pub fn new() -> Self {
        Self {
            errors: vec![],
        }
    }

    pub fn push_spanned<T: ToTokens, U: Display>(&mut self, tokens: T, message: U) {
        self.errors.push(Error::new_spanned(tokens, message));
    }

    pub fn check(self) -> Result<(), Vec<Error>> {
        let errors = self.errors;
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}