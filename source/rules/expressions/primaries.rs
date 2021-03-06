// Tagua VM
//
//
// New BSD License
//
// Copyright © 2016-2017, Ivan Enderlin.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//     * Redistributions of source code must retain the above copyright
//       notice, this list of conditions and the following disclaimer.
//     * Redistributions in binary form must reproduce the above copyright
//       notice, this list of conditions and the following disclaimer in the
//       documentation and/or other materials provided with the distribution.
//     * Neither the name of the Hoa nor the names of its contributors may be
//       used to endorse or promote products derived from this software without
//       specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS AND CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.

//! Group of primary expression rules.
//!
//! The list of all primary expressions is provided by the PHP Language
//! Specification in the [Grammar chapter, Expressions
//! section](https://github.com/php/php-langspec/blob/master/spec/19-grammar.md#primary-expressions).

use super::super::super::ast::{
    AnonymousFunction, Arity, DeclarationScope, DereferencableExpression, Expression, Literal,
    Name, RelativeScope, ScopeResolver, Statement, Ty, Variable,
};
use super::super::super::internal::{Context, Error, ErrorKind};
use super::super::super::tokens;
use super::super::super::tokens::{Span, Token};
use super::super::literals::{literal, string_single_quoted};
use super::super::statements::compound_statement;
use super::super::statements::function::{native_type, parameters};
use super::super::tokens::{name, qualified_name, variable};
use super::expression;
use smallvec::SmallVec;
use std::result::Result as StdResult;

/// Intrinsic errors.
pub enum IntrinsicError {
    /// The exit code is reserved (only 255 is reserved to PHP).
    ReservedExitCode,

    /// The exit code is out of range if greater than 255.
    OutOfRangeExitCode,

    /// The list constructor must contain at least one item.
    ListIsEmpty,
}

named_attr!(
    #[doc="
        Recognize all kind of primary expressions.

        # Examples

        ```
        use std::borrow::Cow;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::primary;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            primary(Span::new(b\"echo 'Hello, World!'\")),
            Ok((
                Span::new_at(b\"\", 20, 1, 21),
                Expression::Echo(vec![
                    Expression::Literal(
                        Literal::String(
                            Token::new(
                                Cow::from(&b\"Hello, World!\"[..]),
                                Span::new_at(b\"'Hello, World!'\", 5, 1, 6)
                            )
                        )
                    )
                ])
            ))
        );
        # }
        ```
    "],
    pub primary<Span, Expression>,
    alt_complete!(
        class_constant_access
      | variable              => { variable_mapper }
      | constant_access       => { constant_access_mapper }
      | literal               => { literal_mapper }
      | array
      | intrinsic
      | anonymous_function
      | preceded!(
            tag!(tokens::LEFT_PARENTHESIS),
            terminated!(
                first!(expression),
                first!(tag!(tokens::RIGHT_PARENTHESIS))
            )
        )
    )
);

#[inline]
fn variable_mapper(variable: Variable) -> Expression {
    Expression::Variable(variable)
}

#[inline]
fn constant_access_mapper(name: Name) -> Expression {
    Expression::Name(name)
}

#[inline]
fn literal_mapper(literal: Literal) -> Expression {
    Expression::Literal(literal)
}

named_attr!(
    #[doc="
        Recognize a class constant access.
    "],
    pub class_constant_access<Span, Expression>,
    do_parse!(
        scope: terminated!(
            scope_resolution_qualifier,
            first!(tag!(tokens::STATIC_CALL))
        ) >>
        name: first!(name) >>
        ( class_constant_access_mapper(scope, name) )
    )
);

#[inline]
fn class_constant_access_mapper<'a>(scope: ScopeResolver<'a>, name: Span<'a>) -> Expression<'a> {
    Expression::ClassConstantAccess(scope, name)
}

named_attr!(
    #[doc="
        Recognize a constant access.

        This parser is an alias to the `qualified_name` parser.
    "],
    pub constant_access<Span, Name>,
    call!(qualified_name)
);

named_attr!(
    #[doc="
        Recognize a scope resolution qualifier.
    "],
    pub scope_resolution_qualifier<Span, ScopeResolver>,
    alt!(
        relative_scope            => { scope_resolution_relative_mapper }
      | qualified_name            => { scope_resolution_name_mapper }
      | dereferencable_expression => { scope_resolution_dereferencable_mapper }
    )
);

#[inline]
fn scope_resolution_relative_mapper<'a>(scope: RelativeScope) -> ScopeResolver<'a> {
    ScopeResolver::ByRelative(scope)
}

#[inline]
fn scope_resolution_name_mapper<'a>(name: Name<'a>) -> ScopeResolver<'a> {
    ScopeResolver::ByName(name)
}

#[inline]
fn scope_resolution_dereferencable_mapper<'a>(
    expression: DereferencableExpression<'a>,
) -> ScopeResolver<'a> {
    ScopeResolver::ByExpression(expression)
}

named_attr!(
    #[doc="
        Recognize a dereferencable expression.
    "],
    pub dereferencable_expression<Span, DereferencableExpression>,
    alt!(
        variable => { dereferencable_variable_mapper }
      | preceded!(
            tag!(tokens::LEFT_PARENTHESIS),
            terminated!(
                first!(expression),
                first!(tag!(tokens::RIGHT_PARENTHESIS))
            )
        )                    => { dereferencable_sub_expression_mapper }
      | array                => { dereferencable_array_mapper }
      | string_single_quoted => { dereferencable_string_mapper }
    )
);

#[inline]
fn dereferencable_variable_mapper<'a>(variable: Variable<'a>) -> DereferencableExpression<'a> {
    DereferencableExpression::Variable(variable)
}

#[inline]
fn dereferencable_sub_expression_mapper<'a>(
    expression: Expression<'a>,
) -> DereferencableExpression<'a> {
    DereferencableExpression::Expression(Box::new(expression))
}

#[inline]
fn dereferencable_array_mapper<'a>(array: Expression<'a>) -> DereferencableExpression<'a> {
    DereferencableExpression::Array(Box::new(array))
}

#[inline]
fn dereferencable_string_mapper<'a>(string: Literal<'a>) -> DereferencableExpression<'a> {
    DereferencableExpression::String(string)
}

named_attr!(
    #[doc="
        Recognize a scope resolution qualifier.

        # Examples

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::RelativeScope;
        use tagua_parser::rules::expressions::primaries::relative_scope;
        use tagua_parser::tokens::Span;

        # fn main() {
        assert_eq!(
            relative_scope(Span::new(b\"self\")),
            Ok((
                Span::new_at(b\"\", 4, 1, 5),
                RelativeScope::ToSelf
            ))
        );
        # }
        ```
    "],
    pub relative_scope<Span, RelativeScope>,
    alt!(
        tag!(tokens::SELF)   => { |_| { RelativeScope::ToSelf } }
      | tag!(tokens::PARENT) => { |_| { RelativeScope::ToParent } }
      | tag!(tokens::STATIC) => { |_| { RelativeScope::ToStatic } }
    )
);

named_attr!(
    #[doc="
        Recognize an array.

        # Examples

        ```
        use std::borrow::Cow;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal, Variable};
        use tagua_parser::rules::expressions::primaries::array;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            array(Span::new(b\"[42, 'foo' => $bar]\")),
            Ok((
                Span::new_at(b\"\", 19, 1, 20),
                Expression::Array(vec![
                    (
                        None,
                        Expression::Literal(Literal::Integer(Token::new(42i64, Span::new_at(b\"42\", 1, 1, 2))))
                    ),
                    (
                        Some(Expression::Literal(Literal::String(Token::new(Cow::from(&b\"foo\"[..]), Span::new_at(b\"'foo'\", 5, 1, 6))))),
                        Expression::Variable(Variable(Span::new_at(b\"bar\", 15, 1, 16)))
                    )
                ])
            ))
        );
        # }
        ```
    "],
    pub array<Span, Expression>,
    alt!(
        preceded!(
            tag!(tokens::LEFT_SQUARE_BRACKET),
            alt!(
                map_res!(
                    first!(tag!(tokens::RIGHT_SQUARE_BRACKET)),
                    empty_array_mapper
                )
              | terminated!(
                    array_pairs,
                    first!(tag!(tokens::RIGHT_SQUARE_BRACKET))
                )
            )
        )
      | preceded!(
            preceded!(
                keyword!(tokens::ARRAY),
                first!(tag!(tokens::LEFT_PARENTHESIS))
            ),
            alt!(
                map_res!(
                    first!(tag!(tokens::RIGHT_PARENTHESIS)),
                    empty_array_mapper
                )
              | terminated!(
                    array_pairs,
                    first!(tag!(tokens::RIGHT_PARENTHESIS))
                )
            )
        )
    )
);

named!(
    array_pairs<Span, Expression>,
    do_parse!(
        accumulator: map_res!(
            first!(array_pair),
            into_vector_mapper
        ) >>
        result: fold_into_vector_many0!(
            preceded!(
                first!(tag!(tokens::COMMA)),
                first!(array_pair)
            ),
            accumulator
        ) >>
        opt!(first!(tag!(tokens::COMMA))) >>
        (into_array(result))
    )
);

named!(
    array_pair<Span, (Option<Expression>, Expression)>,
    do_parse!(
        key: opt!(
            terminated!(
                expression,
                first!(tag!(tokens::MAP))
            )
        ) >>
        value: alt!(
            map_res!(
                preceded!(
                    first!(tag!(tokens::REFERENCE)),
                    first!(expression)
                ),
                value_by_reference_array_mapper
            )
          | first!(expression)
        ) >>
        ((key, value))
    )
);

#[inline]
fn empty_array_mapper(_: Span) -> StdResult<Expression, ()> {
    Ok(Expression::Array(vec![]))
}

#[inline]
fn value_by_reference_array_mapper(expression: Expression) -> StdResult<Expression, ()> {
    Ok(Expression::Reference(Box::new(expression)))
}

#[inline]
fn into_array<'a>(expressions: Vec<(Option<Expression<'a>>, Expression<'a>)>) -> Expression<'a> {
    Expression::Array(expressions)
}

named_attr!(
    #[doc="
        Recognize all kind of intrinsics.

        # Examples

        ```
        use std::borrow::Cow;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::intrinsic;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            intrinsic(Span::new(b\"echo 'Hello, World!'\")),
            Ok((
                Span::new_at(b\"\", 20, 1, 21),
                Expression::Echo(vec![
                    Expression::Literal(
                        Literal::String(
                            Token::new(
                                Cow::from(&b\"Hello, World!\"[..]),
                                Span::new_at(b\"'Hello, World!'\", 5, 1, 6)
                            )
                        )
                    )
                ])
            ))
        );
        # }
        ```
    "],
    pub intrinsic<Span, Expression>,
    alt!(
        intrinsic_construct
      | intrinsic_operator
    )
);

named!(
    intrinsic_construct<Span, Expression>,
    alt!(
        intrinsic_echo
      | intrinsic_list
      | intrinsic_unset
    )
);

named!(
    intrinsic_operator<Span, Expression>,
    alt!(
        intrinsic_empty
      | intrinsic_eval
      | intrinsic_exit
      | intrinsic_isset
      | intrinsic_print
    )
);

named_attr!(
    #[doc="
        Recognize an echo.

        # Examples

        ```
        use std::borrow::Cow;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::intrinsic_echo;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            intrinsic_echo(Span::new(b\"echo 'Hello,', ' World!'\")),
            Ok((
                Span::new_at(b\"\", 24, 1, 25),
                Expression::Echo(vec![
                    Expression::Literal(Literal::String(Token::new(Cow::from(&b\"Hello,\"[..]), Span::new_at(b\"'Hello,'\", 5, 1, 6)))),
                    Expression::Literal(Literal::String(Token::new(Cow::from(&b\" World!\"[..]), Span::new_at(b\"' World!'\", 15, 1, 16))))
                ])
            ))
        );
        # }
        ```
    "],
    pub intrinsic_echo<Span, Expression>,
    do_parse!(
        accumulator: map_res!(
            preceded!(
                keyword!(tokens::ECHO),
                first!(expression)
            ),
            into_vector_mapper
        ) >>
        result: fold_into_vector_many0!(
            preceded!(
                first!(tag!(tokens::COMMA)),
                first!(expression)
            ),
            accumulator
        ) >>
        (into_echo(result))
    )
);

#[inline]
fn into_vector_mapper<T>(item: T) -> StdResult<Vec<T>, ()> {
    Ok(vec![item])
}

#[inline]
fn into_echo(expressions: Vec<Expression>) -> Expression {
    Expression::Echo(expressions)
}

named_attr!(
    #[doc="
        Recognize a list.

        # Examples

        ```
        use std::borrow::Cow;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal, Variable};
        use tagua_parser::rules::expressions::primaries::intrinsic_list;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            intrinsic_list(Span::new(b\"list('foo' => $foo, 'bar' => $bar)\")),
            Ok((
                Span::new_at(b\"\", 34, 1, 35),
                Expression::List(vec![
                    Some((
                        Some(Expression::Literal(Literal::String(Token::new(Cow::from(&b\"foo\"[..]), Span::new_at(b\"'foo'\", 5, 1, 6))))),
                        Expression::Variable(Variable(Span::new_at(b\"foo\", 15, 1, 16)))
                    )),
                    Some((
                        Some(Expression::Literal(Literal::String(Token::new(Cow::from(&b\"bar\"[..]), Span::new_at(b\"'bar'\", 20, 1, 21))))),
                        Expression::Variable(Variable(Span::new_at(b\"bar\", 30, 1, 31)))
                    ))
                ])
            ))
        );
        # }
        ```
    "],
    pub intrinsic_list<Span, Expression>,
    map_res_and_input!(
        preceded!(
            preceded!(
                keyword!(tokens::LIST),
                first!(tag!(tokens::LEFT_PARENTHESIS))
            ),
            terminated!(
                alt!(
                    intrinsic_keyed_list
                  | intrinsic_unkeyed_list
                ),
                first!(tag!(tokens::RIGHT_PARENTHESIS))
            )
        ),
        intrinsic_list_mapper
    )
);

named!(
    intrinsic_keyed_list<Span, Expression>,
    do_parse!(
        accumulator: map_res!(
            first!(intrinsic_keyed_list_item),
            into_vector_mapper
        ) >>
        result: fold_into_vector_many0!(
            preceded!(
                first!(tag!(tokens::COMMA)),
                first!(intrinsic_keyed_list_item)
            ),
            accumulator
        ) >>
        opt!(first!(tag!(tokens::COMMA))) >>
        (into_list(result))
    )
);

named!(
    intrinsic_unkeyed_list<Span, Expression>,
    do_parse!(
        accumulator: map_res!(
            opt!(first!(intrinsic_unkeyed_list_item)),
            into_vector_mapper
        ) >>
        result: fold_into_vector_many0!(
            preceded!(
                first!(tag!(tokens::COMMA)),
                opt!(first!(intrinsic_unkeyed_list_item))
            ),
            accumulator
        ) >>
        (into_list(result))
    )
);

named!(
    intrinsic_keyed_list_item<Span, Option<(Option<Expression>, Expression)>>,
    do_parse!(
        key: terminated!(
            expression,
            first!(tag!(tokens::MAP))
        ) >>
        value: first!(expression) >>
        (Some((Some(key), value)))
    )
);

named!(
    intrinsic_unkeyed_list_item<Span, (Option<Expression>, Expression)>,
    do_parse!(
        value: expression >>
        ((None, value))
    )
);

#[inline]
fn into_list<'a>(
    expressions: Vec<Option<(Option<Expression<'a>>, Expression<'a>)>>,
) -> Expression<'a> {
    Expression::List(expressions)
}

#[inline]
fn intrinsic_list_mapper<'a, 'b>(
    expression: Expression<'a>,
    input: Span<'b>,
) -> StdResult<Expression<'a>, Error<Span<'b>>> {
    match expression {
        Expression::List(items) => {
            if items.iter().any(|item| item.is_some()) {
                Ok(Expression::List(items))
            } else {
                Err(Error::Error(Context::Code(
                    input,
                    ErrorKind::Custom(IntrinsicError::ListIsEmpty as u32),
                )))
            }
        }

        _ => Ok(expression),
    }
}

named_attr!(
    #[doc="
        Recognize an unset.

        # Examples

        ```
        # extern crate smallvec;
        # #[macro_use]
        # extern crate tagua_parser;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Variable};
        use tagua_parser::rules::expressions::primaries::intrinsic_unset;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            intrinsic_unset(Span::new(b\"unset($foo, $bar)\")),
            Ok((
                Span::new_at(b\"\", 17, 1, 18),
                Expression::Unset(smallvec![
                    Variable(Span::new_at(b\"foo\", 7, 1, 8)),
                    Variable(Span::new_at(b\"bar\", 13, 1, 14))
                ])
            ))
        );
        # }
        ```
    "],
    pub intrinsic_unset<Span, Expression>,
    do_parse!(
        accumulator: map_res!(
            preceded!(
                keyword!(tokens::UNSET),
                preceded!(
                    first!(tag!(tokens::LEFT_PARENTHESIS)),
                    first!(variable)
                )
            ),
            into_smallvector_mapper
        ) >>
        result: terminated!(
            fold_into_vector_many0!(
                preceded!(
                    first!(tag!(tokens::COMMA)),
                    first!(variable)
                ),
                accumulator
            ),
            first!(tag!(tokens::RIGHT_PARENTHESIS))
        ) >>
        (into_unset(result))
    )
);

#[inline]
fn into_smallvector_mapper(variable: Variable) -> StdResult<SmallVec<[Variable; 1]>, ()> {
    Ok(smallvec![variable])
}

#[inline]
fn into_unset(variables: SmallVec<[Variable; 1]>) -> Expression {
    Expression::Unset(variables)
}

named_attr!(
    #[doc="
        Recognize an empty.

        # Examples

        ```
        use std::borrow::Cow;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::intrinsic_empty;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            intrinsic_empty(Span::new(b\"empty('foo')\")),
            Ok((
                Span::new_at(b\"\", 12, 1, 13),
                Expression::Empty(
                    Box::new(
                        Expression::Literal(
                            Literal::String(
                                Token::new(Cow::from(&b\"foo\"[..]), Span::new_at(b\"'foo'\", 6, 1, 7))
                            )
                        )
                    )
                )
            ))
        );
        # }
        ```
    "],
    pub intrinsic_empty<Span, Expression>,
    map_res!(
        preceded!(
            keyword!(tokens::EMPTY),
            preceded!(
                first!(tag!(tokens::LEFT_PARENTHESIS)),
                terminated!(
                    first!(expression),
                    first!(tag!(tokens::RIGHT_PARENTHESIS))
                )
            )
        ),
        empty_mapper
    )
);

#[inline]
fn empty_mapper(expression: Expression) -> StdResult<Expression, ()> {
    Ok(Expression::Empty(Box::new(expression)))
}

named_attr!(
    #[doc="
        Recognize an lazy evaluation.

        # Examples

        ```
        use std::borrow::Cow;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::intrinsic_eval;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            intrinsic_eval(Span::new(b\"eval('1 + 2')\")),
            Ok((
                Span::new_at(b\"\", 13, 1, 14),
                Expression::Eval(
                    Box::new(
                        Expression::Literal(
                            Literal::String(Token::new(Cow::from(&b\"1 + 2\"[..]), Span::new_at(b\"'1 + 2'\", 5, 1, 6)))
                        )
                    )
                )
            ))
        );
        # }
        ```
    "],
    pub intrinsic_eval<Span, Expression>,
    map_res!(
        preceded!(
            keyword!(tokens::EVAL),
            preceded!(
                first!(tag!(tokens::LEFT_PARENTHESIS)),
                terminated!(
                    first!(expression),
                    first!(tag!(tokens::RIGHT_PARENTHESIS))
                )
            )
        ),
        eval_mapper
    )
);

#[inline]
fn eval_mapper(expression: Expression) -> StdResult<Expression, ()> {
    Ok(Expression::Eval(Box::new(expression)))
}

named_attr!(
    #[doc="
        Recognize an exit.

        # Examples

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::intrinsic_exit;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            intrinsic_exit(Span::new(b\"exit(7)\")),
            Ok((
                Span::new_at(b\"\", 7, 1, 8),
                Expression::Exit(
                    Some(
                        Box::new(
                            Expression::Literal(
                                Literal::Integer(Token::new(7i64, Span::new_at(b\"7\", 5, 1, 6)))
                            )
                        )
                    )
                )
            ))
        );
        # }
        ```
    "],
    pub intrinsic_exit<Span, Expression>,
    map_res_and_input!(
        preceded!(
            alt!(
                keyword!(tokens::EXIT)
              | keyword!(tokens::DIE)
            ),
            opt!(
                preceded!(
                    first!(tag!(tokens::LEFT_PARENTHESIS)),
                    terminated!(
                        first!(expression),
                        first!(tag!(tokens::RIGHT_PARENTHESIS))
                    )
                )
            )
        ),
        exit_mapper
    )
);

#[inline]
fn exit_mapper<'a, 'b>(
    expression: Option<Expression<'a>>,
    input: Span<'b>,
) -> StdResult<Expression<'a>, Error<Span<'b>>> {
    match expression {
        Some(expression) => {
            if let Expression::Literal(Literal::Integer(Token { value: code, .. })) = expression {
                if code == 255 {
                    return Err(Error::Error(Context::Code(
                        input,
                        ErrorKind::Custom(IntrinsicError::ReservedExitCode as u32),
                    )));
                } else if code > 255 {
                    return Err(Error::Error(Context::Code(
                        input,
                        ErrorKind::Custom(IntrinsicError::OutOfRangeExitCode as u32),
                    )));
                }
            }

            Ok(Expression::Exit(Some(Box::new(expression))))
        }

        None => Ok(Expression::Exit(None)),
    }
}

named_attr!(
    #[doc="
        Recognize an exit.

        # Examples

        ```
        # extern crate smallvec;
        # #[macro_use]
        # extern crate tagua_parser;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Variable};
        use tagua_parser::rules::expressions::primaries::intrinsic_isset;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            intrinsic_isset(Span::new(b\"isset($foo, $bar)\")),
            Ok((
                Span::new_at(b\"\", 17, 1, 18),
                Expression::Isset(smallvec![
                    Variable(Span::new_at(b\"foo\", 7, 1, 8)),
                    Variable(Span::new_at(b\"bar\", 13, 1, 14))
                ])
            ))
        );
        # }
        ```
    "],
    pub intrinsic_isset<Span, Expression>,
    do_parse!(
        accumulator: map_res!(
            preceded!(
                keyword!(tokens::ISSET),
                preceded!(
                    first!(tag!(tokens::LEFT_PARENTHESIS)),
                    first!(variable)
                )
            ),
            into_smallvector_mapper
        ) >>
        result: terminated!(
            fold_into_vector_many0!(
                preceded!(
                    first!(tag!(tokens::COMMA)),
                    first!(variable)
                ),
                accumulator
            ),
            first!(tag!(tokens::RIGHT_PARENTHESIS))
        ) >>
        (into_isset(result))
    )
);

#[inline]
fn into_isset(variables: SmallVec<[Variable; 1]>) -> Expression {
    Expression::Isset(variables)
}

named_attr!(
    #[doc="
        Recognize a print.

        # Examples

        ```
        use std::borrow::Cow;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::intrinsic_print;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            intrinsic_print(Span::new(b\"print('Hello, World!')\")),
            Ok((
                Span::new_at(b\"\", 22, 1, 23),
                Expression::Print(
                    Box::new(
                        Expression::Literal(
                            Literal::String(
                                Token::new(Cow::from(&b\"Hello, World!\"[..]), Span::new_at(b\"'Hello, World!'\", 6, 1, 7))
                            )
                        )
                    )
                )
            ))
        );
        # }
        ```
    "],
    pub intrinsic_print<Span, Expression>,
    map_res!(
        preceded!(
            keyword!(tokens::PRINT),
            first!(expression)
        ),
        print_mapper
    )
);

#[inline]
fn print_mapper(expression: Expression) -> StdResult<Expression, ()> {
    Ok(Expression::Print(Box::new(expression)))
}

named_attr!(
    #[doc="
        Recognize an anonymous function.

        # Examples

        ```
        # extern crate smallvec;
        # #[macro_use]
        # extern crate tagua_parser;
        use tagua_parser::Result;
        use tagua_parser::ast::{
            AnonymousFunction,
            Arity,
            Expression,
            Name,
            Parameter,
            DeclarationScope,
            Statement,
            Ty,
            Variable
        };
        use tagua_parser::rules::expressions::primaries::anonymous_function;
        use tagua_parser::tokens::{
            Span,
            Token
        };

        # fn main() {
        assert_eq!(
            anonymous_function(Span::new(b\"function &($x, \\\\I\\\\J $y, int &$z) use ($a, &$b): O { return; }\")),
            Ok((
                Span::new_at(b\"\", 61, 1, 62),
                Expression::AnonymousFunction(
                    AnonymousFunction {
                        declaration_scope: DeclarationScope::Dynamic,
                        inputs           : Arity::Finite(vec![
                            Parameter {
                                ty   : Ty::Copy(None),
                                name : Variable(Span::new_at(b\"x\", 12, 1, 13)),
                                value: None
                            },
                            Parameter {
                                ty   : Ty::Copy(Some(Name::FullyQualified(smallvec![Span::new_at(b\"I\", 16, 1, 17), Span::new_at(b\"J\", 18, 1, 19)]))),
                                name : Variable(Span::new_at(b\"y\", 21, 1, 22)),
                                value: None
                            },
                            Parameter {
                                ty   : Ty::Reference(Some(Name::FullyQualified(smallvec![Span::new_at(b\"int\", 24, 1, 25)]))),
                                name : Variable(Span::new_at(b\"z\", 30, 1, 31)),
                                value: None
                            }
                        ]),
                        output         : Ty::Reference(Some(Name::Unqualified(Span::new_at(b\"O\", 48, 1, 49)))),
                        enclosing_scope: Some(vec![
                            Expression::Variable(Variable(Span::new_at(b\"a\", 39, 1, 40))),
                            Expression::Reference(
                                Box::new(
                                    Expression::Variable(Variable(Span::new_at(b\"b\", 44, 1, 45)))
                                )
                            )
                        ]),
                        body: vec![Statement::Return]
                    }
                )
            ))
        );
        # }
        ```
    "],
    pub anonymous_function<Span, Expression>,
    do_parse!(
        static_scope: opt!(keyword!(tokens::STATIC)) >>
        first!(keyword!(tokens::FUNCTION)) >>
        output_is_a_reference: opt!(first!(tag!(tokens::REFERENCE))) >>
        inputs: first!(parameters) >>
        enclosing_scope: opt!(first!(anonymous_function_use)) >>
        output_type: opt!(
            preceded!(
                first!(tag!(tokens::FUNCTION_OUTPUT)),
                alt!(
                    first!(native_type)
                  | first!(qualified_name)
                )
            )
        ) >>
        body: first!(compound_statement) >>
        (
            into_anonymous_function(
                match static_scope {
                    Some(_) => {
                        DeclarationScope::Static
                    },

                    None => {
                        DeclarationScope::Dynamic
                    }
                },
                output_is_a_reference.is_some(),
                inputs,
                output_type,
                enclosing_scope,
                body
            )
        )
    )
);

named!(
    anonymous_function_use<Span, Vec<Expression>>,
    map_res!(
        terminated!(
            preceded!(
                keyword!(tokens::USE),
                preceded!(
                    first!(tag!(tokens::LEFT_PARENTHESIS)),
                    opt!(first!(anonymous_function_use_list))
                )
            ),
            first!(tag!(tokens::RIGHT_PARENTHESIS))
        ),
        anonymous_function_use_mapper
    )
);

#[inline]
fn anonymous_function_use_mapper(
    enclosing_list: Option<Vec<Expression>>,
) -> StdResult<Vec<Expression>, ()> {
    match enclosing_list {
        Some(enclosing_list) => Ok(enclosing_list),

        None => Ok(vec![]),
    }
}

named!(
    anonymous_function_use_list<Span, Vec<Expression>>,
    do_parse!(
        accumulator: map_res!(
            anonymous_function_use_list_item,
            into_vector_mapper
        ) >>
        result: fold_into_vector_many0!(
            preceded!(
                first!(tag!(tokens::COMMA)),
                first!(anonymous_function_use_list_item)
            ),
            accumulator
        ) >>
        (result)
    )
);

named!(
    anonymous_function_use_list_item<Span, Expression>,
    do_parse!(
        reference: opt!(first!(tag!(tokens::REFERENCE))) >>
        name: first!(variable) >>
        (into_anonymous_function_use_list_item(reference.is_some(), name))
    )
);

#[inline]
fn into_anonymous_function_use_list_item(reference: bool, name: Variable) -> Expression {
    if reference {
        Expression::Reference(Box::new(Expression::Variable(name)))
    } else {
        Expression::Variable(name)
    }
}

#[inline]
fn into_anonymous_function<'a>(
    declaration_scope: DeclarationScope,
    output_is_a_reference: bool,
    inputs: Arity<'a>,
    output_type: Option<Name<'a>>,
    enclosing_scope: Option<Vec<Expression<'a>>>,
    body: Vec<Statement<'a>>,
) -> Expression<'a> {
    let output = if output_is_a_reference {
        Ty::Reference(output_type)
    } else {
        Ty::Copy(output_type)
    };

    Expression::AnonymousFunction(AnonymousFunction {
        declaration_scope: declaration_scope,
        inputs: inputs,
        output: output,
        enclosing_scope: enclosing_scope,
        body: body,
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::super::ast::{
        AnonymousFunction, Arity, DeclarationScope, DereferencableExpression, Expression, Literal,
        Name, Parameter, RelativeScope, ScopeResolver, Statement, Ty, Variable,
    };
    use super::super::super::super::internal::{Context, Error, ErrorKind};
    use super::super::super::super::tokens::{Span, Token};
    use super::super::expression;
    use super::{
        anonymous_function, array, class_constant_access, dereferencable_expression, intrinsic,
        intrinsic_construct, intrinsic_echo, intrinsic_empty, intrinsic_eval, intrinsic_exit,
        intrinsic_isset, intrinsic_list, intrinsic_operator, intrinsic_print, intrinsic_unset,
        primary, relative_scope, scope_resolution_qualifier,
    };
    use std::borrow::Cow;

    #[test]
    fn case_class_constant_access_relative_self() {
        let input = Span::new(b"self::FOO");
        let output = Ok((
            Span::new_at(b"", 9, 1, 10),
            Expression::ClassConstantAccess(
                ScopeResolver::ByRelative(RelativeScope::ToSelf),
                Span::new_at(b"FOO", 6, 1, 7),
            ),
        ));

        assert_eq!(class_constant_access(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_class_constant_access_relative_parent() {
        let input = Span::new(b"parent::FOO");
        let output = Ok((
            Span::new_at(b"", 11, 1, 12),
            Expression::ClassConstantAccess(
                ScopeResolver::ByRelative(RelativeScope::ToParent),
                Span::new_at(b"FOO", 8, 1, 9),
            ),
        ));

        assert_eq!(class_constant_access(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_class_constant_access_relative_static() {
        let input = Span::new(b"static::FOO");
        let output = Ok((
            Span::new_at(b"", 11, 1, 12),
            Expression::ClassConstantAccess(
                ScopeResolver::ByRelative(RelativeScope::ToStatic),
                Span::new_at(b"FOO", 8, 1, 9),
            ),
        ));

        assert_eq!(class_constant_access(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_class_constant_access_qualified_name() {
        let input = Span::new(b"Foo\\Bar::BAZ");
        let output = Ok((
            Span::new_at(b"", 12, 1, 13),
            Expression::ClassConstantAccess(
                ScopeResolver::ByName(Name::Qualified(smallvec![
                    Span::new(b"Foo"),
                    Span::new_at(b"Bar", 4, 1, 5)
                ])),
                Span::new_at(b"BAZ", 9, 1, 10),
            ),
        ));

        assert_eq!(class_constant_access(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_class_constant_access_dereferencable_expression() {
        let input = Span::new(b"$this::FOO");
        let output = Ok((
            Span::new_at(b"", 10, 1, 11),
            Expression::ClassConstantAccess(
                ScopeResolver::ByExpression(DereferencableExpression::Variable(Variable(
                    Span::new_at(b"this", 2, 1, 3),
                ))),
                Span::new_at(b"FOO", 7, 1, 8),
            ),
        ));

        assert_eq!(class_constant_access(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_scope_resolution_qualifier_by_relative() {
        let input = Span::new(b"self");
        let output = Ok((
            Span::new_at(b"", 4, 1, 5),
            ScopeResolver::ByRelative(RelativeScope::ToSelf),
        ));

        assert_eq!(scope_resolution_qualifier(input), output);
    }

    #[test]
    fn case_scope_resolution_qualifier_by_name() {
        let input = Span::new(b"Foo");
        let output = Ok((
            Span::new_at(b"", 3, 1, 4),
            ScopeResolver::ByName(Name::Unqualified(Span::new(b"Foo"))),
        ));

        assert_eq!(scope_resolution_qualifier(input), output);
    }

    #[test]
    fn case_scope_resolution_qualifier_by_dereferencable_expression() {
        let input = Span::new(b"$foo");
        let output = Ok((
            Span::new_at(b"", 4, 1, 5),
            ScopeResolver::ByExpression(DereferencableExpression::Variable(Variable(
                Span::new_at(b"foo", 1, 1, 2),
            ))),
        ));

        assert_eq!(scope_resolution_qualifier(input), output);
    }

    #[test]
    fn case_dereferencable_expression_variable() {
        let input = Span::new(b"$foo");
        let output = Ok((
            Span::new_at(b"", 4, 1, 5),
            DereferencableExpression::Variable(Variable(Span::new_at(b"foo", 1, 1, 2))),
        ));

        assert_eq!(dereferencable_expression(input), output);
    }

    #[test]
    fn case_dereferencable_expression_sub_expression() {
        let input = Span::new(b"($foo)");
        let output = Ok((
            Span::new_at(b"", 6, 1, 7),
            DereferencableExpression::Expression(Box::new(Expression::Variable(Variable(
                Span::new_at(b"foo", 2, 1, 3),
            )))),
        ));

        assert_eq!(dereferencable_expression(input), output);
    }

    #[test]
    fn case_dereferencable_expression_array() {
        let input = Span::new(b"['C', 'f']");
        let output = Ok((
            Span::new_at(b"", 10, 1, 11),
            DereferencableExpression::Array(Box::new(Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"C"[..]),
                        Span::new_at(b"'C'", 1, 1, 2),
                    ))),
                ),
                (
                    None,
                    Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"f"[..]),
                        Span::new_at(b"'f'", 6, 1, 7),
                    ))),
                ),
            ]))),
        ));

        assert_eq!(dereferencable_expression(input), output);
    }

    #[test]
    fn case_dereferencable_expression_string() {
        let input = Span::new(b"'C'");
        let output = Ok((
            Span::new_at(b"", 3, 1, 4),
            DereferencableExpression::String(Literal::String(Token::new(
                Cow::from(&b"C"[..]),
                Span::new_at(b"'C'", 0, 1, 1),
            ))),
        ));

        assert_eq!(dereferencable_expression(input), output);
    }

    #[test]
    fn case_relative_scope_self() {
        let input = Span::new(b"self");
        let output = Ok((Span::new_at(b"", 4, 1, 5), RelativeScope::ToSelf));

        assert_eq!(relative_scope(input), output);
    }

    #[test]
    fn case_relative_scope_parent() {
        let input = Span::new(b"parent");
        let output = Ok((Span::new_at(b"", 6, 1, 7), RelativeScope::ToParent));

        assert_eq!(relative_scope(input), output);
    }

    #[test]
    fn case_relative_scope_static() {
        let input = Span::new(b"static");
        let output = Ok((Span::new_at(b"", 6, 1, 7), RelativeScope::ToStatic));

        assert_eq!(relative_scope(input), output);
    }

    #[test]
    fn case_array_empty() {
        let input = Span::new(b"[ /* foo */ ]");
        let output = Ok((Span::new_at(b"", 13, 1, 14), Expression::Array(vec![])));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_one_value() {
        let input = Span::new(b"['foo']");
        let output = Ok((
            Span::new_at(b"", 7, 1, 8),
            Expression::Array(vec![(
                None,
                Expression::Literal(Literal::String(Token::new(
                    Cow::from(&b"foo"[..]),
                    Span::new_at(b"'foo'", 1, 1, 2),
                ))),
            )]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_one_pair() {
        let input = Span::new(b"[42 => 'foo']");
        let output = Ok((
            Span::new_at(b"", 13, 1, 14),
            Expression::Array(vec![(
                Some(Expression::Literal(Literal::Integer(Token::new(
                    42i64,
                    Span::new_at(b"42", 1, 1, 2),
                )))),
                Expression::Literal(Literal::String(Token::new(
                    Cow::from(&b"foo"[..]),
                    Span::new_at(b"'foo'", 7, 1, 8),
                ))),
            )]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_many_pairs() {
        let input = Span::new(b"['foo', 42 => 'bar', 'baz' => $qux]");
        let output = Ok((
            Span::new_at(b"", 35, 1, 36),
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"foo"[..]),
                        Span::new_at(b"'foo'", 1, 1, 2),
                    ))),
                ),
                (
                    Some(Expression::Literal(Literal::Integer(Token::new(
                        42i64,
                        Span::new_at(b"42", 8, 1, 9),
                    )))),
                    Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"bar"[..]),
                        Span::new_at(b"'bar'", 14, 1, 15),
                    ))),
                ),
                (
                    Some(Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"baz"[..]),
                        Span::new_at(b"'baz'", 21, 1, 22),
                    )))),
                    Expression::Variable(Variable(Span::new_at(b"qux", 31, 1, 32))),
                ),
            ]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_vector_capacity() {
        if let Ok((_, Expression::Array(vector))) = array(Span::new(b"[1, 2, 3]")) {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_array_trailing_comma() {
        let input = Span::new(b"[1, 2, 3, /* foo */]");
        let output = Ok((
            Span::new_at(b"", 20, 1, 21),
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::Integer(Token::new(
                        1i64,
                        Span::new_at(b"1", 1, 1, 2),
                    ))),
                ),
                (
                    None,
                    Expression::Literal(Literal::Integer(Token::new(
                        2i64,
                        Span::new_at(b"2", 4, 1, 5),
                    ))),
                ),
                (
                    None,
                    Expression::Literal(Literal::Integer(Token::new(
                        3i64,
                        Span::new_at(b"3", 7, 1, 8),
                    ))),
                ),
            ]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_recursive() {
        let input = Span::new(b"['foo', 42 => [3 => 5, 7 => [11 => '13']], 'baz' => $qux]");
        let output = Ok((
            Span::new_at(b"", 57, 1, 58),
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"foo"[..]),
                        Span::new_at(b"'foo'", 1, 1, 2),
                    ))),
                ),
                (
                    Some(Expression::Literal(Literal::Integer(Token::new(
                        42i64,
                        Span::new_at(b"42", 8, 1, 9),
                    )))),
                    Expression::Array(vec![
                        (
                            Some(Expression::Literal(Literal::Integer(Token::new(
                                3i64,
                                Span::new_at(b"3", 15, 1, 16),
                            )))),
                            Expression::Literal(Literal::Integer(Token::new(
                                5i64,
                                Span::new_at(b"5", 20, 1, 21),
                            ))),
                        ),
                        (
                            Some(Expression::Literal(Literal::Integer(Token::new(
                                7i64,
                                Span::new_at(b"7", 23, 1, 24),
                            )))),
                            Expression::Array(vec![(
                                Some(Expression::Literal(Literal::Integer(Token::new(
                                    11i64,
                                    Span::new_at(b"11", 29, 1, 30),
                                )))),
                                Expression::Literal(Literal::String(Token::new(
                                    Cow::from(&b"13"[..]),
                                    Span::new_at(b"'13'", 35, 1, 36),
                                ))),
                            )]),
                        ),
                    ]),
                ),
                (
                    Some(Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"baz"[..]),
                        Span::new_at(b"'baz'", 43, 1, 44),
                    )))),
                    Expression::Variable(Variable(Span::new_at(b"qux", 53, 1, 54))),
                ),
            ]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_value_by_reference() {
        let input = Span::new(b"[7 => &$foo, 42 => $bar]");
        let output = Ok((
            Span::new_at(b"", 24, 1, 25),
            Expression::Array(vec![
                (
                    Some(Expression::Literal(Literal::Integer(Token::new(
                        7i64,
                        Span::new_at(b"7", 1, 1, 2),
                    )))),
                    Expression::Reference(Box::new(Expression::Variable(Variable(Span::new_at(
                        b"foo", 8, 1, 9,
                    ))))),
                ),
                (
                    Some(Expression::Literal(Literal::Integer(Token::new(
                        42i64,
                        Span::new_at(b"42", 13, 1, 14),
                    )))),
                    Expression::Variable(Variable(Span::new_at(b"bar", 20, 1, 21))),
                ),
            ]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_array_trailing_commas() {
        let input = Span::new(b"[1, 2, 3,,]");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            array(input),
            Err(Error::Error(Context::Code(input, ErrorKind::Alt)))
        );
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_array_empty_trailing_comma() {
        let input = Span::new(b"[,]");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            array(input),
            Err(Error::Error(Context::Code(input, ErrorKind::Alt)))
        );
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_empty() {
        let input = Span::new(b"array ( /* foo */ )");
        let output = Ok((Span::new_at(b"", 19, 1, 20), Expression::Array(vec![])));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_one_value() {
        let input = Span::new(b"array('foo')");
        let output = Ok((
            Span::new_at(b"", 12, 1, 13),
            Expression::Array(vec![(
                None,
                Expression::Literal(Literal::String(Token::new(
                    Cow::from(&b"foo"[..]),
                    Span::new_at(b"'foo'", 6, 1, 7),
                ))),
            )]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_one_pair() {
        let input = Span::new(b"array(42 => 'foo')");
        let output = Ok((
            Span::new_at(b"", 18, 1, 19),
            Expression::Array(vec![(
                Some(Expression::Literal(Literal::Integer(Token::new(
                    42i64,
                    Span::new_at(b"42", 6, 1, 7),
                )))),
                Expression::Literal(Literal::String(Token::new(
                    Cow::from(&b"foo"[..]),
                    Span::new_at(b"'foo'", 12, 1, 13),
                ))),
            )]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_many_pairs() {
        let input = Span::new(b"array('foo', 42 => 'bar', 'baz' => $qux)");
        let output = Ok((
            Span::new_at(b"", 40, 1, 41),
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"foo"[..]),
                        Span::new_at(b"'foo'", 6, 1, 7),
                    ))),
                ),
                (
                    Some(Expression::Literal(Literal::Integer(Token::new(
                        42i64,
                        Span::new_at(b"42", 13, 1, 14),
                    )))),
                    Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"bar"[..]),
                        Span::new_at(b"'bar'", 19, 1, 20),
                    ))),
                ),
                (
                    Some(Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"baz"[..]),
                        Span::new_at(b"'baz'", 26, 1, 27),
                    )))),
                    Expression::Variable(Variable(Span::new_at(b"qux", 36, 1, 37))),
                ),
            ]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_trailing_comma() {
        let input = Span::new(b"array(1, 2, 3, /* foo */)");
        let output = Ok((
            Span::new_at(b"", 25, 1, 26),
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::Integer(Token::new(
                        1i64,
                        Span::new_at(b"1", 6, 1, 7),
                    ))),
                ),
                (
                    None,
                    Expression::Literal(Literal::Integer(Token::new(
                        2i64,
                        Span::new_at(b"2", 9, 1, 10),
                    ))),
                ),
                (
                    None,
                    Expression::Literal(Literal::Integer(Token::new(
                        3i64,
                        Span::new_at(b"3", 12, 1, 13),
                    ))),
                ),
            ]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_recursive() {
        let input =
            Span::new(b"array('foo', 42 => array(3 => 5, 7 => array(11 => '13')), 'baz' => $qux)");
        let output = Ok((
            Span::new_at(b"", 72, 1, 73),
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"foo"[..]),
                        Span::new_at(b"'foo'", 6, 1, 7),
                    ))),
                ),
                (
                    Some(Expression::Literal(Literal::Integer(Token::new(
                        42i64,
                        Span::new_at(b"42", 13, 1, 14),
                    )))),
                    Expression::Array(vec![
                        (
                            Some(Expression::Literal(Literal::Integer(Token::new(
                                3i64,
                                Span::new_at(b"3", 25, 1, 26),
                            )))),
                            Expression::Literal(Literal::Integer(Token::new(
                                5i64,
                                Span::new_at(b"5", 30, 1, 31),
                            ))),
                        ),
                        (
                            Some(Expression::Literal(Literal::Integer(Token::new(
                                7i64,
                                Span::new_at(b"7", 33, 1, 34),
                            )))),
                            Expression::Array(vec![(
                                Some(Expression::Literal(Literal::Integer(Token::new(
                                    11i64,
                                    Span::new_at(b"11", 44, 1, 45),
                                )))),
                                Expression::Literal(Literal::String(Token::new(
                                    Cow::from(&b"13"[..]),
                                    Span::new_at(b"'13'", 50, 1, 51),
                                ))),
                            )]),
                        ),
                    ]),
                ),
                (
                    Some(Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"baz"[..]),
                        Span::new_at(b"'baz'", 58, 1, 59),
                    )))),
                    Expression::Variable(Variable(Span::new_at(b"qux", 68, 1, 69))),
                ),
            ]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_value_by_reference() {
        let input = Span::new(b"array(7 => &$foo, 42 => $bar)");
        let output = Ok((
            Span::new_at(b"", 29, 1, 30),
            Expression::Array(vec![
                (
                    Some(Expression::Literal(Literal::Integer(Token::new(
                        7i64,
                        Span::new_at(b"7", 6, 1, 7),
                    )))),
                    Expression::Reference(Box::new(Expression::Variable(Variable(Span::new_at(
                        b"foo", 13, 1, 14,
                    ))))),
                ),
                (
                    Some(Expression::Literal(Literal::Integer(Token::new(
                        42i64,
                        Span::new_at(b"42", 18, 1, 19),
                    )))),
                    Expression::Variable(Variable(Span::new_at(b"bar", 25, 1, 26))),
                ),
            ]),
        ));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_variable() {
        let input = Span::new(b"$foo");
        let output = Ok((
            Span::new_at(b"", 4, 1, 5),
            Expression::Variable(Variable(Span::new_at(b"foo", 1, 1, 2))),
        ));

        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_qualified_name() {
        let input = Span::new(b"Foo\\Bar");
        let output = Ok((
            Span::new_at(b"", 7, 1, 8),
            Expression::Name(Name::Qualified(smallvec![
                Span::new(b"Foo"),
                Span::new_at(b"Bar", 4, 1, 5)
            ])),
        ));

        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_literal() {
        let input = Span::new(b"'Hello, World!'");
        let output = Ok((
            Span::new_at(b"", 15, 1, 16),
            Expression::Literal(Literal::String(Token::new(
                Cow::from(&b"Hello, World!"[..]),
                input,
            ))),
        ));

        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_echo_one_expression() {
        let input = Span::new(b"echo /* baz */ 'foobar'");
        let output = Ok((
            Span::new_at(b"", 23, 1, 24),
            Expression::Echo(vec![Expression::Literal(Literal::String(Token::new(
                Cow::from(&b"foobar"[..]),
                Span::new_at(b"'foobar'", 15, 1, 16),
            )))]),
        ));

        assert_eq!(intrinsic_echo(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_echo_many_expressions() {
        let input = Span::new(b"echo /* baz */ 'foobar',\t $bazqux, \n  42");
        let output = Ok((
            Span::new_at(b"", 40, 2, 5),
            Expression::Echo(vec![
                Expression::Literal(Literal::String(Token::new(
                    Cow::from(&b"foobar"[..]),
                    Span::new_at(b"'foobar'", 15, 1, 16),
                ))),
                Expression::Variable(Variable(Span::new_at(b"bazqux", 27, 1, 28))),
                Expression::Literal(Literal::Integer(Token::new(
                    42i64,
                    Span::new_at(b"42", 38, 2, 3),
                ))),
            ]),
        ));

        assert_eq!(intrinsic_echo(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_echo_vector_capacity() {
        if let Ok((_, Expression::Echo(vector))) =
            intrinsic_echo(Span::new(b"echo 'foobar', $bazqux, 42"))
        {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_invalid_intrinsic_echo_expression_missing() {
        let input = Span::new(b"echo;");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_echo(input),
            Err(Error::Error(Context::Code(
                Span::new_at(b";", 4, 1, 5),
                ErrorKind::Alt,
            )))
        );
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_keyed_one_pattern() {
        let input = Span::new(b"list('foo' => $foo)");
        let output = Ok((
            Span::new_at(b"", 19, 1, 20),
            Expression::List(vec![Some((
                Some(Expression::Literal(Literal::String(Token::new(
                    Cow::from(&b"foo"[..]),
                    Span::new_at(b"'foo'", 5, 1, 6),
                )))),
                Expression::Variable(Variable(Span::new_at(b"foo", 15, 1, 16))),
            ))]),
        ));

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_keyed_many_patterns() {
        let input = Span::new(b"list('foo' => $foo, 'bar' => $bar, 'baz' => $baz)");
        let output = Ok((
            Span::new_at(b"", 49, 1, 50),
            Expression::List(vec![
                Some((
                    Some(Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"foo"[..]),
                        Span::new_at(b"'foo'", 5, 1, 6),
                    )))),
                    Expression::Variable(Variable(Span::new_at(b"foo", 15, 1, 16))),
                )),
                Some((
                    Some(Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"bar"[..]),
                        Span::new_at(b"'bar'", 20, 1, 21),
                    )))),
                    Expression::Variable(Variable(Span::new_at(b"bar", 30, 1, 31))),
                )),
                Some((
                    Some(Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"baz"[..]),
                        Span::new_at(b"'baz'", 35, 1, 36),
                    )))),
                    Expression::Variable(Variable(Span::new_at(b"baz", 45, 1, 46))),
                )),
            ]),
        ));

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_keyed_vector_capacity() {
        if let Ok((_, Expression::List(vector))) = intrinsic_list(Span::new(
            b"list('foo' => $foo, 'bar' => $bar, 'baz' => $baz)",
        )) {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_intrinsic_list_keyed_trailing_comma() {
        let input = Span::new(b"list('foo' => $foo, 'bar' => $bar,)");
        let output = Ok((
            Span::new_at(b"", 35, 1, 36),
            Expression::List(vec![
                Some((
                    Some(Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"foo"[..]),
                        Span::new_at(b"'foo'", 5, 1, 6),
                    )))),
                    Expression::Variable(Variable(Span::new_at(b"foo", 15, 1, 16))),
                )),
                Some((
                    Some(Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"bar"[..]),
                        Span::new_at(b"'bar'", 20, 1, 21),
                    )))),
                    Expression::Variable(Variable(Span::new_at(b"bar", 30, 1, 31))),
                )),
            ]),
        ));

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_keyed_recursive() {
        let input = Span::new(b"list('foo' => list('bar' => $bar), 'baz' => list(, $qux))");
        let output = Ok((
            Span::new_at(b"", 57, 1, 58),
            Expression::List(vec![
                Some((
                    Some(Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"foo"[..]),
                        Span::new_at(b"'foo'", 5, 1, 6),
                    )))),
                    Expression::List(vec![Some((
                        Some(Expression::Literal(Literal::String(Token::new(
                            Cow::from(&b"bar"[..]),
                            Span::new_at(b"'bar'", 19, 1, 20),
                        )))),
                        Expression::Variable(Variable(Span::new_at(b"bar", 29, 1, 30))),
                    ))]),
                )),
                Some((
                    Some(Expression::Literal(Literal::String(Token::new(
                        Cow::from(&b"baz"[..]),
                        Span::new_at(b"'baz'", 35, 1, 36),
                    )))),
                    Expression::List(vec![
                        None,
                        Some((
                            None,
                            Expression::Variable(Variable(Span::new_at(b"qux", 52, 1, 53))),
                        )),
                    ]),
                )),
            ]),
        ));

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_unkeyed_one_pattern() {
        let input = Span::new(b"list($foo)");
        let output = Ok((
            Span::new_at(b"", 10, 1, 11),
            Expression::List(vec![Some((
                None,
                Expression::Variable(Variable(Span::new_at(b"foo", 6, 1, 7))),
            ))]),
        ));

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_unkeyed_many_patterns() {
        let input = Span::new(b"list($foo, $bar, $baz)");
        let output = Ok((
            Span::new_at(b"", 22, 1, 23),
            Expression::List(vec![
                Some((
                    None,
                    Expression::Variable(Variable(Span::new_at(b"foo", 6, 1, 7))),
                )),
                Some((
                    None,
                    Expression::Variable(Variable(Span::new_at(b"bar", 12, 1, 13))),
                )),
                Some((
                    None,
                    Expression::Variable(Variable(Span::new_at(b"baz", 18, 1, 19))),
                )),
            ]),
        ));

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_unkeyed_vector_capacity() {
        if let Ok((_, Expression::List(vector))) =
            intrinsic_list(Span::new(b"list($foo, $bar, $baz)"))
        {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_intrinsic_list_unkeyed_free_patterns() {
        let input = Span::new(b"list($foo, , $bar, , , $baz,)");
        let output = Ok((
            Span::new_at(b"", 29, 1, 30),
            Expression::List(vec![
                Some((
                    None,
                    Expression::Variable(Variable(Span::new_at(b"foo", 6, 1, 7))),
                )),
                None,
                Some((
                    None,
                    Expression::Variable(Variable(Span::new_at(b"bar", 14, 1, 15))),
                )),
                None,
                None,
                Some((
                    None,
                    Expression::Variable(Variable(Span::new_at(b"baz", 24, 1, 25))),
                )),
                None,
            ]),
        ));

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_unkeyed_recursive() {
        let input = Span::new(b"list($foo, list($bar), list('baz' => $baz))");
        let output = Ok((
            Span::new_at(b"", 43, 1, 44),
            Expression::List(vec![
                Some((
                    None,
                    Expression::Variable(Variable(Span::new_at(b"foo", 6, 1, 7))),
                )),
                Some((
                    None,
                    Expression::List(vec![Some((
                        None,
                        Expression::Variable(Variable(Span::new_at(b"bar", 17, 1, 18))),
                    ))]),
                )),
                Some((
                    None,
                    Expression::List(vec![Some((
                        Some(Expression::Literal(Literal::String(Token::new(
                            Cow::from(&b"baz"[..]),
                            Span::new_at(b"'baz'", 28, 1, 29),
                        )))),
                        Expression::Variable(Variable(Span::new_at(b"baz", 38, 1, 39))),
                    ))]),
                )),
            ]),
        ));

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_list_mixed_pairs() {
        let input = Span::new(b"list('foo' => $foo, $bar)");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_list(input),
            Err(Error::Error(Context::Code(
                Span::new_at(b"$bar)", 20, 1, 21),
                ErrorKind::Tag
            )))
        );
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_list_empty() {
        let input = Span::new(b"list()");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_list(input),
            Err(Error::Error(Context::Code(input, ErrorKind::MapRes)))
        );
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_list_only_free_patterns() {
        let input = Span::new(b"list(,,,)");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_list(input),
            Err(Error::Error(Context::Code(input, ErrorKind::MapRes)))
        );
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_unset_one_variable() {
        let input = Span::new(b"unset($foo)");
        let output = Ok((
            Span::new_at(b"", 11, 1, 12),
            Expression::Unset(smallvec![Variable(Span::new_at(b"foo", 7, 1, 8))]),
        ));

        assert_eq!(intrinsic_unset(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_unset_many_variables() {
        let input = Span::new(b"unset($foo, $bar, $baz)");
        let output = Ok((
            Span::new_at(b"", 23, 1, 24),
            Expression::Unset(smallvec![
                Variable(Span::new_at(b"foo", 7, 1, 8)),
                Variable(Span::new_at(b"bar", 13, 1, 14)),
                Variable(Span::new_at(b"baz", 19, 1, 20))
            ]),
        ));

        assert_eq!(intrinsic_unset(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_unset_vector_capacity() {
        if let Ok((_, Expression::Unset(vector))) =
            intrinsic_unset(Span::new(b"unset($foo, $bar, $baz)"))
        {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_invalid_intrinsic_unset_zero_variable() {
        let input = Span::new(b"unset()");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_unset(input),
            Err(Error::Error(Context::Code(
                Span::new_at(b")", 6, 1, 7),
                ErrorKind::Tag
            )))
        );
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_empty_string() {
        let input = Span::new(b"empty('foo')");
        let output = Ok((
            Span::new_at(b"", 12, 1, 13),
            Expression::Empty(Box::new(Expression::Literal(Literal::String(Token::new(
                Cow::from(&b"foo"[..]),
                Span::new_at(b"'foo'", 6, 1, 7),
            ))))),
        ));

        assert_eq!(intrinsic_empty(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_empty_integer() {
        let input = Span::new(b"empty(42)");
        let output = Ok((
            Span::new_at(b"", 9, 1, 10),
            Expression::Empty(Box::new(Expression::Literal(Literal::Integer(Token::new(
                42i64,
                Span::new_at(b"42", 6, 1, 7),
            ))))),
        ));

        assert_eq!(intrinsic_empty(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_empty_expression_missing() {
        let input = Span::new(b"empty()");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_empty(input),
            Err(Error::Error(Context::Code(
                Span::new_at(b")", 6, 1, 7),
                ErrorKind::Alt
            )))
        );
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_eval() {
        let input = Span::new(b"eval('1 + 2;')");
        let output = Ok((
            Span::new_at(b"", 14, 1, 15),
            Expression::Eval(Box::new(Expression::Literal(Literal::String(Token::new(
                Cow::from(&b"1 + 2;"[..]),
                Span::new_at(b"'1 + 2;'", 5, 1, 6),
            ))))),
        ));

        assert_eq!(intrinsic_eval(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_eval_expression_missing() {
        let input = Span::new(b"eval()");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_eval(input),
            Err(Error::Error(Context::Code(
                Span::new_at(b")", 5, 1, 6),
                ErrorKind::Alt
            )))
        );
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_exit() {
        let input = Span::new(b"exit(42)");
        let output = Ok((
            Span::new_at(b"", 8, 1, 9),
            Expression::Exit(Some(Box::new(Expression::Literal(Literal::Integer(
                Token::new(42i64, Span::new_at(b"42", 5, 1, 6)),
            ))))),
        ));

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_exit_with_no_argument() {
        let input = Span::new(b"exit 42");
        let output = Ok((Span::new_at(b" 42", 4, 1, 5), Expression::Exit(None)));

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_exit_with_a_variable() {
        let input = Span::new(b"exit($foo)");
        let output = Ok((
            Span::new_at(b"", 10, 1, 11),
            Expression::Exit(Some(Box::new(Expression::Variable(Variable(
                Span::new_at(b"foo", 6, 1, 7),
            ))))),
        ));

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_exit_with_reserved_code_255() {
        let input = Span::new(b"exit(255)");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_exit(input),
            Err(Error::Error(Context::Code(input, ErrorKind::MapRes)))
        );
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_exit_with_out_of_range_code() {
        let input = Span::new(b"exit(256)");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_exit(input),
            Err(Error::Error(Context::Code(input, ErrorKind::MapRes)))
        );
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_die() {
        let input = Span::new(b"die(42)");
        let output = Ok((
            Span::new_at(b"", 7, 1, 8),
            Expression::Exit(Some(Box::new(Expression::Literal(Literal::Integer(
                Token::new(42i64, Span::new_at(b"42", 4, 1, 5)),
            ))))),
        ));

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_die_with_no_parenthesis() {
        let input = Span::new(b"die 42");
        let output = Ok((Span::new_at(b" 42", 3, 1, 4), Expression::Exit(None)));

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_die_with_a_variable() {
        let input = Span::new(b"die($foo)");
        let output = Ok((
            Span::new_at(b"", 9, 1, 10),
            Expression::Exit(Some(Box::new(Expression::Variable(Variable(
                Span::new_at(b"foo", 5, 1, 6),
            ))))),
        ));

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_die_with_reserved_code_255() {
        let input = Span::new(b"die(255)");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_exit(input),
            Err(Error::Error(Context::Code(input, ErrorKind::MapRes)))
        );
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_die_with_out_of_range_code() {
        let input = Span::new(b"die(256)");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_exit(input),
            Err(Error::Error(Context::Code(input, ErrorKind::MapRes)))
        );
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_isset_one_variable() {
        let input = Span::new(b"isset($foo)");
        let output = Ok((
            Span::new_at(b"", 11, 1, 12),
            Expression::Isset(smallvec![Variable(Span::new_at(b"foo", 7, 1, 8))]),
        ));

        assert_eq!(intrinsic_isset(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_isset_many_variables() {
        let input = Span::new(b"isset($foo, $bar, $baz)");
        let output = Ok((
            Span::new_at(b"", 23, 1, 24),
            Expression::Isset(smallvec![
                Variable(Span::new_at(b"foo", 7, 1, 8)),
                Variable(Span::new_at(b"bar", 13, 1, 14)),
                Variable(Span::new_at(b"baz", 19, 1, 20))
            ]),
        ));

        assert_eq!(intrinsic_isset(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_isset_vector_capacity() {
        if let Ok((_, Expression::Isset(vector))) =
            intrinsic_isset(Span::new(b"isset($foo, $bar, $baz)"))
        {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_invalid_intrinsic_isset_zero_variable() {
        let input = Span::new(b"isset()");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_isset(input),
            Err(Error::Error(Context::Code(
                Span::new_at(b")", 6, 1, 7),
                ErrorKind::Tag
            )))
        );
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_print() {
        let input = Span::new(b"print /* baz */ 'foobar'");
        let output = Ok((
            Span::new_at(b"", 24, 1, 25),
            Expression::Print(Box::new(Expression::Literal(Literal::String(Token::new(
                Cow::from(&b"foobar"[..]),
                Span::new_at(b"'foobar'", 16, 1, 17),
            ))))),
        ));

        assert_eq!(intrinsic_print(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_print_expression_missing() {
        let input = Span::new(b"print;");
        let output = Err(Error::Error(Context::Code(input, ErrorKind::Alt)));

        assert_eq!(
            intrinsic_print(input),
            Err(Error::Error(Context::Code(
                Span::new_at(b";", 5, 1, 6),
                ErrorKind::Alt
            )))
        );
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_grouped_by_parenthesis() {
        let input = Span::new(b"print (((('foobar'))))");
        let output = Ok((
            Span::new_at(b"", 22, 1, 23),
            Expression::Print(Box::new(Expression::Literal(Literal::String(Token::new(
                Cow::from(&b"foobar"[..]),
                Span::new_at(b"'foobar'", 10, 1, 11),
            ))))),
        ));

        assert_eq!(intrinsic_print(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function() {
        let input = Span::new(b"function (I $x, J &$y) use ($z): O { return; }");
        let output = Ok((
            Span::new_at(b"", 46, 1, 47),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Finite(vec![
                    Parameter {
                        ty: Ty::Copy(Some(Name::Unqualified(Span::new_at(b"I", 10, 1, 11)))),
                        name: Variable(Span::new_at(b"x", 13, 1, 14)),
                        value: None,
                    },
                    Parameter {
                        ty: Ty::Reference(Some(Name::Unqualified(Span::new_at(b"J", 16, 1, 17)))),
                        name: Variable(Span::new_at(b"y", 20, 1, 21)),
                        value: None,
                    },
                ]),
                output: Ty::Copy(Some(Name::Unqualified(Span::new_at(b"O", 33, 1, 34)))),
                enclosing_scope: Some(vec![Expression::Variable(Variable(Span::new_at(
                    b"z", 29, 1, 30,
                )))]),
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_zero() {
        let input = Span::new(b"function () {}");
        let output = Ok((
            Span::new_at(b"", 14, 1, 15),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Constant,
                output: Ty::Copy(None),
                enclosing_scope: None,
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_one_by_copy() {
        let input = Span::new(b"function ($x) {}");
        let output = Ok((
            Span::new_at(b"", 16, 1, 17),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Finite(vec![Parameter {
                    ty: Ty::Copy(None),
                    name: Variable(Span::new_at(b"x", 11, 1, 12)),
                    value: None,
                }]),
                output: Ty::Copy(None),
                enclosing_scope: None,
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_one_by_reference() {
        let input = Span::new(b"function (&$x) {}");
        let output = Ok((
            Span::new_at(b"", 17, 1, 18),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Finite(vec![Parameter {
                    ty: Ty::Reference(None),
                    name: Variable(Span::new_at(b"x", 12, 1, 13)),
                    value: None,
                }]),
                output: Ty::Copy(None),
                enclosing_scope: None,
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_one_with_a_copy_type() {
        let input = Span::new(b"function (A\\B\\C $x) {}");
        let output = Ok((
            Span::new_at(b"", 22, 1, 23),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Finite(vec![Parameter {
                    ty: Ty::Copy(Some(Name::Qualified(smallvec![
                        Span::new_at(b"A", 10, 1, 11),
                        Span::new_at(b"B", 12, 1, 13),
                        Span::new_at(b"C", 14, 1, 15)
                    ]))),
                    name: Variable(Span::new_at(b"x", 17, 1, 18)),
                    value: None,
                }]),
                output: Ty::Copy(None),
                enclosing_scope: None,
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_one_with_a_reference_type() {
        let input = Span::new(b"function (int &$x) {}");
        let output = Ok((
            Span::new_at(b"", 21, 1, 22),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Finite(vec![Parameter {
                    ty: Ty::Reference(Some(Name::FullyQualified(smallvec![Span::new_at(
                        b"int", 10, 1, 11
                    )]))),
                    name: Variable(Span::new_at(b"x", 16, 1, 17)),
                    value: None,
                }]),
                output: Ty::Copy(None),
                enclosing_scope: None,
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_many() {
        let input = Span::new(b"function ($a, I\\J $b, int &$c, \\K $d) {}");
        let output = Ok((
            Span::new_at(b"", 40, 1, 41),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Finite(vec![
                    Parameter {
                        ty: Ty::Copy(None),
                        name: Variable(Span::new_at(b"a", 11, 1, 12)),
                        value: None,
                    },
                    Parameter {
                        ty: Ty::Copy(Some(Name::Qualified(smallvec![
                            Span::new_at(b"I", 14, 1, 15),
                            Span::new_at(b"J", 16, 1, 17)
                        ]))),
                        name: Variable(Span::new_at(b"b", 19, 1, 20)),
                        value: None,
                    },
                    Parameter {
                        ty: Ty::Reference(Some(Name::FullyQualified(smallvec![Span::new_at(
                            b"int", 22, 1, 23
                        )]))),
                        name: Variable(Span::new_at(b"c", 28, 1, 29)),
                        value: None,
                    },
                    Parameter {
                        ty: Ty::Copy(Some(Name::FullyQualified(smallvec![Span::new_at(
                            b"K", 32, 1, 33
                        )]))),
                        name: Variable(Span::new_at(b"d", 35, 1, 36)),
                        value: None,
                    },
                ]),
                output: Ty::Copy(None),
                enclosing_scope: None,
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_output_by_copy() {
        let input = Span::new(b"function (): \\O {}");
        let output = Ok((
            Span::new_at(b"", 18, 1, 19),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Constant,
                output: Ty::Copy(Some(Name::FullyQualified(smallvec![Span::new_at(
                    b"O", 14, 1, 15
                )]))),
                enclosing_scope: None,
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_output_by_reference() {
        let input = Span::new(b"function &(): int {}");
        let output = Ok((
            Span::new_at(b"", 20, 1, 21),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Constant,
                output: Ty::Reference(Some(Name::FullyQualified(smallvec![Span::new_at(
                    b"int", 14, 1, 15
                )]))),
                enclosing_scope: None,
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_empty_enclosing_scope() {
        let input = Span::new(b"function () use () {}");
        let output = Ok((
            Span::new_at(b"", 21, 1, 22),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Constant,
                output: Ty::Copy(None),
                enclosing_scope: Some(vec![]),
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_one_enclosed_variable_by_copy() {
        let input = Span::new(b"function () use ($x) {}");
        let output = Ok((
            Span::new_at(b"", 23, 1, 24),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Constant,
                output: Ty::Copy(None),
                enclosing_scope: Some(vec![Expression::Variable(Variable(Span::new_at(
                    b"x", 18, 1, 19,
                )))]),
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_one_enclosed_variable_by_reference() {
        let input = Span::new(b"function () use (&$x) {}");
        let output = Ok((
            Span::new_at(b"", 24, 1, 25),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Constant,
                output: Ty::Copy(None),
                enclosing_scope: Some(vec![Expression::Reference(Box::new(Expression::Variable(
                    Variable(Span::new_at(b"x", 19, 1, 20)),
                )))]),
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_many_enclosed_variables() {
        let input = Span::new(b"function () use ($x, &$y, $z) {}");
        let output = Ok((
            Span::new_at(b"", 32, 1, 33),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Dynamic,
                inputs: Arity::Constant,
                output: Ty::Copy(None),
                enclosing_scope: Some(vec![
                    Expression::Variable(Variable(Span::new_at(b"x", 18, 1, 19))),
                    Expression::Reference(Box::new(Expression::Variable(Variable(Span::new_at(
                        b"y", 23, 1, 24,
                    ))))),
                    Expression::Variable(Variable(Span::new_at(b"z", 27, 1, 28))),
                ]),
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_static_scope() {
        let input = Span::new(b"static function () {}");
        let output = Ok((
            Span::new_at(b"", 21, 1, 22),
            Expression::AnonymousFunction(AnonymousFunction {
                declaration_scope: DeclarationScope::Static,
                inputs: Arity::Constant,
                output: Ty::Copy(None),
                enclosing_scope: None,
                body: vec![Statement::Return],
            }),
        ));

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }
}
