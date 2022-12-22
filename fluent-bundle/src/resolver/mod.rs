//! The `resolver` module contains the definitions and implementations for the internal
//! `WriteOrResolve` and `WriteOrResolveContext` traits.
//! There is an implementation that resolves AST nodes to a [`FluentValue`], and one
//! that writes to an implementor of the [`std::fmt::Write`] trait.

pub mod errors;
mod expression;
mod inline_expression;
pub(crate) mod pattern;
mod scope;

pub use errors::ResolverError;
use fluent_syntax::ast;
use fluent_syntax::unicode::{unescape_unicode, unescape_unicode_to_string};
pub use scope::Scope;

use std::borrow::{Borrow, Cow};
use std::fmt;

use crate::memoizer::MemoizerKind;
use crate::resource::FluentResource;
use crate::types::FluentValue;

use self::pattern::{resolve_pattern, write_pattern};

pub trait WriteOrResolveContext<'bundle> {
    type Result;

    fn unescape(&mut self, s: &'bundle str) -> Self::Result;
    fn value<'ast, 'args, 'errors, R, M>(
        &mut self,
        scope: &Scope<'bundle, 'ast, 'args, 'errors, R, M>,
        value: Cow<FluentValue<'bundle>>,
    ) -> Self::Result
    where
        R: Borrow<FluentResource>,
        M: MemoizerKind;

    fn error<E: WriteOrResolve<'bundle>>(&mut self, exp: &E, is_ref: bool) -> Self::Result;
    fn resolve_pattern<'ast, 'args, 'errors, R, M>(
        &mut self,
        scope: &mut Scope<'bundle, 'ast, 'args, 'errors, R, M>,
        pattern: &'ast ast::Pattern<&'bundle str>,
    ) -> Self::Result
    where
        R: Borrow<FluentResource>,
        M: MemoizerKind;
}

/// Resolves an AST node to a string that is written to source `W`.
impl<'bundle, W> WriteOrResolveContext<'bundle> for W
where
    W: fmt::Write,
{
    type Result = fmt::Result;

    fn unescape(&mut self, s: &'bundle str) -> Self::Result {
        unescape_unicode(self, s)
    }

    fn value<'ast, 'args, 'errors, R, M>(
        &mut self,
        scope: &Scope<'bundle, 'ast, 'args, 'errors, R, M>,
        value: Cow<FluentValue<'bundle>>,
    ) -> Self::Result
    where
        R: Borrow<FluentResource>,
        M: MemoizerKind,
    {
        assert!(!matches!(value.as_ref(), FluentValue::Error));
        let s = match value {
            Cow::Borrowed(value) => value.as_string(scope),
            Cow::Owned(value) => value.into_string(scope),
        };
        self.write_str(&s)
    }

    fn error<E: WriteOrResolve<'bundle>>(&mut self, exp: &E, is_ref: bool) -> Self::Result {
        if is_ref {
            self.write_char('{')?;
        }
        exp.write_error(self)?;
        if is_ref {
            self.write_char('}')?;
        }
        Ok(())
    }

    fn resolve_pattern<'ast, 'args, 'errors, R, M>(
        &mut self,
        scope: &mut Scope<'bundle, 'ast, 'args, 'errors, R, M>,
        pattern: &'ast ast::Pattern<&'bundle str>,
    ) -> Self::Result
    where
        R: Borrow<FluentResource>,
        M: MemoizerKind,
    {
        write_pattern(pattern, self, scope)
    }
}

/// Resolves an AST node to a [`FluentValue`].
struct ResolveContext;

impl<'bundle> WriteOrResolveContext<'bundle> for ResolveContext {
    type Result = FluentValue<'bundle>;

    fn unescape(&mut self, s: &'bundle str) -> Self::Result {
        unescape_unicode_to_string(s).into()
    }

    fn value<'ast, 'args, 'errors, R, M>(
        &mut self,
        _scope: &Scope<'bundle, 'ast, 'args, 'errors, R, M>,
        value: Cow<FluentValue<'bundle>>,
    ) -> Self::Result
    where
        R: Borrow<FluentResource>,
        M: MemoizerKind,
    {
        value.into_owned()
    }

    fn error<E: WriteOrResolve<'bundle>>(&mut self, _exp: &E, _is_ref: bool) -> Self::Result {
        FluentValue::Error
    }

    fn resolve_pattern<'ast, 'args, 'errors, R, M>(
        &mut self,
        scope: &mut Scope<'bundle, 'ast, 'args, 'errors, R, M>,
        pattern: &'ast ast::Pattern<&'bundle str>,
    ) -> Self::Result
    where
        R: Borrow<FluentResource>,
        M: MemoizerKind,
    {
        resolve_pattern(pattern, scope)
    }
}

pub trait WriteOrResolve<'bundle> {
    fn write_or_resolve<'ast, 'args, 'errors, R, M, T>(
        &'ast self,
        scope: &mut Scope<'bundle, 'ast, 'args, 'errors, R, M>,
        context: &mut T,
    ) -> T::Result
    where
        R: Borrow<FluentResource>,
        M: MemoizerKind,
        T: WriteOrResolveContext<'bundle>;

    /// Writes error information to `W`. This can be used to add FTL errors inline
    /// to a message.
    fn write_error<W>(&self, w: &mut W) -> fmt::Result
    where
        W: fmt::Write;
}
