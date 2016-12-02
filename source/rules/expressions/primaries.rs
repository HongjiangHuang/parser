// Tagua VM
//
//
// New BSD License
//
// Copyright © 2016-2016, Ivan Enderlin.
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

use std::result::Result as StdResult;
use super::expression;
use super::super::literals::literal;
use super::super::statements::compound_statement;
use super::super::statements::function::parameters;
use super::super::tokens::{
    qualified_name,
    variable
};
use super::super::super::ast::{
    AnonymousFunction,
    Arity,
    Expression,
    Literal,
    Name,
    Parameter,
    Scope,
    Statement,
    Ty,
    Variable
};
use super::super::super::internal::{
    Error,
    ErrorKind
};
use super::super::super::tokens;

/// Intrinsic errors.
pub enum IntrinsicError {
    /// The exit code is reserved (only 255 is reserved to PHP).
    ReservedExitCode,

    /// The exit code is out of range if greater than 255.
    OutOfRangeExitCode,

    /// The list constructor must contain at least one item.
    ListIsEmpty
}

named_attr!(
    #[doc="
        Recognize all kind of primary expressions.

        # Examples

        ```
        # extern crate tagua_parser;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::primary;

        # fn main() {
        assert_eq!(
            primary(b\"echo 'Hello, World!'\"),
            Result::Done(
                &b\"\"[..],
                Expression::Echo(vec![
                    Expression::Literal(Literal::String(b\"Hello, World!\".to_vec()))
                ])
            )
        );
        # }
        ```
    "],
    pub primary<Expression>,
    alt!(
        variable       => { variable_mapper }
      | qualified_name => { qualified_name_mapper }
      | literal        => { literal_mapper }
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

#[inline(always)]
fn variable_mapper<'a>(variable: Variable<'a>) -> Expression<'a> {
    Expression::Variable(variable)
}

#[inline(always)]
fn qualified_name_mapper<'a>(name: Name<'a>) -> Expression<'a> {
    Expression::Name(name)
}

#[inline(always)]
fn literal_mapper<'a>(literal: Literal) -> Expression<'a> {
    Expression::Literal(literal)
}

named_attr!(
    #[doc="
        Recognize an array.

        # Examples

        ```
        # extern crate tagua_parser;
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal, Variable};
        use tagua_parser::rules::expressions::primaries::array;

        # fn main() {
        assert_eq!(
            array(b\"[42, 'foo' => $bar]\"),
            Result::Done(
                &b\"\"[..],
                Expression::Array(vec![
                    (
                        None,
                        Expression::Literal(Literal::Integer(42i64))
                    ),
                    (
                        Some(Expression::Literal(Literal::String(b\"foo\".to_vec()))),
                        Expression::Variable(Variable(&b\"bar\"[..]))
                    )
                ])
            )
        );
        # }
        ```
    "],
    pub array<Expression>,
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
    array_pairs<Expression>,
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
    array_pair<(Option<Expression>, Expression)>,
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

#[inline(always)]
fn empty_array_mapper<'a>(_: &[u8]) -> StdResult<Expression<'a>, ()> {
    Ok(Expression::Array(vec![]))
}

#[inline(always)]
fn value_by_reference_array_mapper<'a>(expression: Expression<'a>) -> StdResult<Expression<'a>, ()> {
    Ok(Expression::Reference(Box::new(expression)))
}

#[inline(always)]
fn into_array<'a>(expressions: Vec<(Option<Expression<'a>>, Expression<'a>)>) -> Expression<'a> {
    Expression::Array(expressions)
}

named_attr!(
    #[doc="
        Recognize all kind of intrinsics.

        # Examples

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::intrinsic;

        # fn main() {
        assert_eq!(
            intrinsic(b\"echo 'Hello, World!'\"),
            Result::Done(
                &b\"\"[..],
                Expression::Echo(vec![
                    Expression::Literal(Literal::String(b\"Hello, World!\".to_vec()))
                ])
            )
        );
        # }
        ```
    "],
    pub intrinsic<Expression>,
    alt!(
        intrinsic_construct
      | intrinsic_operator
    )
);

named!(
    intrinsic_construct<Expression>,
    alt!(
        intrinsic_echo
      | intrinsic_list
      | intrinsic_unset
    )
);

named!(
    intrinsic_operator<Expression>,
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
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::intrinsic_echo;

        # fn main() {
        assert_eq!(
            intrinsic_echo(b\"echo 'Hello,', ' World!'\"),
            Result::Done(
                &b\"\"[..],
                Expression::Echo(vec![
                    Expression::Literal(Literal::String(b\"Hello,\".to_vec())),
                    Expression::Literal(Literal::String(b\" World!\".to_vec()))
                ])
            )
        );
        # }
        ```
    "],
    pub intrinsic_echo<Expression>,
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

#[inline(always)]
fn into_vector_mapper<T>(item: T) -> StdResult<Vec<T>, ()> {
    Ok(vec![item])
}

#[inline(always)]
fn into_echo<'a>(expressions: Vec<Expression<'a>>) -> Expression<'a> {
    Expression::Echo(expressions)
}

named_attr!(
    #[doc="
        Recognize a list.

        # Examples

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal, Variable};
        use tagua_parser::rules::expressions::primaries::intrinsic_list;

        # fn main() {
        assert_eq!(
            intrinsic_list(b\"list('foo' => $foo, 'bar' => $bar)\"),
            Result::Done(
                &b\"\"[..],
                Expression::List(vec![
                    Some((
                        Some(Expression::Literal(Literal::String(b\"foo\".to_vec()))),
                        Expression::Variable(Variable(&b\"foo\"[..]))
                    )),
                    Some((
                        Some(Expression::Literal(Literal::String(b\"bar\".to_vec()))),
                        Expression::Variable(Variable(&b\"bar\"[..]))
                    ))
                ])
            )
        );
        # }
        ```
    "],
    pub intrinsic_list<Expression>,
    map_res!(
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
    intrinsic_keyed_list<Expression>,
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
    intrinsic_unkeyed_list<Expression>,
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
    intrinsic_keyed_list_item< Option<(Option<Expression>, Expression)> >,
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
    intrinsic_unkeyed_list_item<(Option<Expression>, Expression)>,
    do_parse!(
        value: expression >>
        ((None, value))
    )
);

#[inline(always)]
fn into_list<'a>(expressions: Vec<Option<(Option<Expression<'a>>, Expression<'a>)>>) -> Expression<'a> {
    Expression::List(expressions)
}

#[inline(always)]
fn intrinsic_list_mapper<'a>(expression: Expression<'a>) -> StdResult<Expression<'a>, Error<ErrorKind>> {
    match expression {
        Expression::List(items) => {
            if items.iter().any(|ref item| item.is_some()) {
                Ok(Expression::List(items))
            } else {
                Err(Error::Code(ErrorKind::Custom(IntrinsicError::ListIsEmpty as u32)))
            }
        },

        _ => {
            Ok(expression)
        }
    }
}

named_attr!(
    #[doc="
        Recognize an unset.

        # Examples

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Variable};
        use tagua_parser::rules::expressions::primaries::intrinsic_unset;

        # fn main() {
        assert_eq!(
            intrinsic_unset(b\"unset($foo, $bar)\"),
            Result::Done(
                &b\"\"[..],
                Expression::Unset(vec![
                    Expression::Variable(Variable(&b\"foo\"[..])),
                    Expression::Variable(Variable(&b\"bar\"[..]))
                ])
            )
        );
        # }
        ```
    "],
    pub intrinsic_unset<Expression>,
    do_parse!(
        accumulator: map_res!(
            preceded!(
                keyword!(tokens::UNSET),
                preceded!(
                    first!(tag!(tokens::LEFT_PARENTHESIS)),
                    first!(expression)
                )
            ),
            into_vector_mapper
        ) >>
        result: terminated!(
            fold_into_vector_many0!(
                preceded!(
                    first!(tag!(tokens::COMMA)),
                    first!(expression)
                ),
                accumulator
            ),
            first!(tag!(tokens::RIGHT_PARENTHESIS))
        ) >>
        (into_unset(result))
    )
);

#[inline(always)]
fn into_unset<'a>(expressions: Vec<Expression<'a>>) -> Expression<'a> {
    Expression::Unset(expressions)
}

named_attr!(
    #[doc="
        Recognize an empty.

        # Examples

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::intrinsic_empty;

        # fn main() {
        assert_eq!(
            intrinsic_empty(b\"empty('foo')\"),
            Result::Done(
                &b\"\"[..],
                Expression::Empty(
                    Box::new(
                        Expression::Literal(
                            Literal::String(b\"foo\".to_vec())
                        )
                    )
                )
            )
        );
        # }
        ```
    "],
    pub intrinsic_empty<Expression>,
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

#[inline(always)]
fn empty_mapper<'a>(expression: Expression<'a>) -> StdResult<Expression<'a>, ()> {
    Ok(Expression::Empty(Box::new(expression)))
}

named_attr!(
    #[doc="
        Recognize an lazy evaluation.

        # Examples

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::intrinsic_eval;

        # fn main() {
        assert_eq!(
            intrinsic_eval(b\"eval('1 + 2')\"),
            Result::Done(
                &b\"\"[..],
                Expression::Eval(
                    Box::new(
                        Expression::Literal(
                            Literal::String(b\"1 + 2\".to_vec())
                        )
                    )
                )
            )
        );
        # }
        ```
    "],
    pub intrinsic_eval<Expression>,
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

#[inline(always)]
fn eval_mapper<'a>(expression: Expression<'a>) -> StdResult<Expression<'a>, ()> {
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

        # fn main() {
        assert_eq!(
            intrinsic_exit(b\"exit(7)\"),
            Result::Done(
                &b\"\"[..],
                Expression::Exit(
                    Some(
                        Box::new(
                            Expression::Literal(
                                Literal::Integer(7i64)
                            )
                        )
                    )
                )
            )
        );
        # }
        ```
    "],
    pub intrinsic_exit<Expression>,
    map_res!(
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

#[inline(always)]
fn exit_mapper<'a>(expression: Option<Expression<'a>>) -> StdResult<Expression<'a>, Error<ErrorKind>> {
    match expression {
        Some(expression) => {
            if let Expression::Literal(Literal::Integer(code)) = expression {
                if code == 255 {
                    return Err(Error::Code(ErrorKind::Custom(IntrinsicError::ReservedExitCode as u32)));
                } else if code > 255 {
                    return Err(Error::Code(ErrorKind::Custom(IntrinsicError::OutOfRangeExitCode as u32)));
                }
            }

            Ok(Expression::Exit(Some(Box::new(expression))))
        },

        None => {
            Ok(Expression::Exit(None))
        }
    }
}

named_attr!(
    #[doc="
        Recognize an exit.

        # Examples

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Variable};
        use tagua_parser::rules::expressions::primaries::intrinsic_isset;

        # fn main() {
        assert_eq!(
            intrinsic_isset(b\"isset($foo, $bar)\"),
            Result::Done(
                &b\"\"[..],
                Expression::Isset(vec![
                    Expression::Variable(Variable(&b\"foo\"[..])),
                    Expression::Variable(Variable(&b\"bar\"[..]))
                ])
            )
        );
        # }
        ```
    "],
    pub intrinsic_isset<Expression>,
    do_parse!(
        accumulator: map_res!(
            preceded!(
                keyword!(tokens::ISSET),
                preceded!(
                    first!(tag!(tokens::LEFT_PARENTHESIS)),
                    first!(expression)
                )
            ),
            into_vector_mapper
        ) >>
        result: terminated!(
            fold_into_vector_many0!(
                preceded!(
                    first!(tag!(tokens::COMMA)),
                    first!(expression)
                ),
                accumulator
            ),
            first!(tag!(tokens::RIGHT_PARENTHESIS))
        ) >>
        (into_isset(result))
    )
);

#[inline(always)]
fn into_isset<'a>(expressions: Vec<Expression<'a>>) -> Expression<'a> {
    Expression::Isset(expressions)
}

named_attr!(
    #[doc="
        Recognize a print.

        # Examples

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{Expression, Literal};
        use tagua_parser::rules::expressions::primaries::intrinsic_print;

        # fn main() {
        assert_eq!(
            intrinsic_print(b\"print('Hello, World!')\"),
            Result::Done(
                &b\"\"[..],
                Expression::Print(
                    Box::new(
                        Expression::Literal(Literal::String(b\"Hello, World!\".to_vec()))
                    )
                )
            )
        );
        # }
        ```
    "],
    pub intrinsic_print<Expression>,
    map_res!(
        preceded!(
            keyword!(tokens::PRINT),
            first!(expression)
        ),
        print_mapper
    )
);

#[inline(always)]
fn print_mapper<'a>(expression: Expression<'a>) -> StdResult<Expression<'a>, ()> {
    Ok(Expression::Print(Box::new(expression)))
}

named_attr!(
    #[doc="
        Recognize an anonymous function.

        # Examples

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{
            AnonymousFunction,
            Arity,
            Expression,
            Name,
            Parameter,
            Scope,
            Statement,
            Ty,
            Variable
        };
        use tagua_parser::rules::expressions::primaries::anonymous_function;

        # fn main() {
        assert_eq!(
            anonymous_function(b\"function &($x, \\\\I\\\\J $y, int &$z): O use ($a, &$b) { return; }\"),
            Result::Done(
                &b\"\"[..],
                Expression::AnonymousFunction(
                    AnonymousFunction {
                        declaration_scope: Scope::Dynamic,
                        inputs           : Arity::Finite(vec![
                            Parameter {
                                ty   : Ty::Copy(None),
                                name : Variable(&b\"x\"[..]),
                                value: None
                            },
                            Parameter {
                                ty   : Ty::Copy(Some(Name::FullyQualified(vec![&b\"I\"[..], &b\"J\"[..]]))),
                                name : Variable(&b\"y\"[..]),
                                value: None
                            },
                            Parameter {
                                ty   : Ty::Reference(Some(Name::Unqualified(&b\"int\"[..]))),
                                name : Variable(&b\"z\"[..]),
                                value: None
                            }
                        ]),
                        output         : Ty::Reference(Some(Name::Unqualified(&b\"O\"[..]))),
                        enclosing_scope: Some(vec![
                            Expression::Variable(Variable(&b\"a\"[..])),
                            Expression::Reference(
                                Box::new(
                                    Expression::Variable(Variable(&b\"b\"[..]))
                                )
                            )
                        ]),
                        body: vec![Statement::Return]
                    }
                )
            )
        );
        # }
        ```
    "],
    pub anonymous_function<Expression>,
    do_parse!(
        static_scope: opt!(keyword!(tokens::STATIC)) >>
        first!(keyword!(tokens::FUNCTION)) >>
        output_is_a_reference: opt!(first!(tag!(tokens::REFERENCE))) >>
        first!(tag!(tokens::LEFT_PARENTHESIS)) >>
        inputs: opt!(first!(parameters)) >>
        first!(tag!(tokens::RIGHT_PARENTHESIS)) >>
        output_type: opt!(
            preceded!(
                first!(tag!(tokens::FUNCTION_OUTPUT)),
                first!(qualified_name)
            )
        ) >>
        enclosing_scope: opt!(first!(anonymous_function_use)) >>
        body: first!(compound_statement) >>
        (
            into_anonymous_function(
                match static_scope {
                    Some(_) => {
                        Scope::Static
                    },

                    None => {
                        Scope::Dynamic
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
    anonymous_function_use< Vec<Expression> >,
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

#[inline(always)]
fn anonymous_function_use_mapper<'a>(enclosing_list: Option<Vec<Expression<'a>>>) -> StdResult<Vec<Expression<'a>>, ()> {
    match enclosing_list {
        Some(enclosing_list) => {
            Ok(enclosing_list)
        },

        None => {
            Ok(vec![])
        }
    }
}

named!(
    anonymous_function_use_list< Vec<Expression> >,
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
    anonymous_function_use_list_item<Expression>,
    do_parse!(
        reference: opt!(first!(tag!(tokens::REFERENCE))) >>
        name: first!(variable) >>
        (into_anonymous_function_use_list_item(reference.is_some(), name))
    )
);

#[inline(always)]
fn into_anonymous_function_use_list_item<'a>(reference: bool, name: Variable<'a>) -> Expression<'a> {
    if reference {
        Expression::Reference(Box::new(Expression::Variable(name)))
    } else {
        Expression::Variable(name)
    }
}

#[inline(always)]
fn into_anonymous_function<'a>(
    declaration_scope    : Scope,
    output_is_a_reference: bool,
    inputs               : Option<Vec<Parameter<'a>>>,
    output_type          : Option<Name<'a>>,
    enclosing_scope      : Option<Vec<Expression<'a>>>,
    body                 : Vec<Statement<'a>>
) -> Expression<'a> {
    let inputs = match inputs {
        Some(inputs) => {
            Arity::Finite(inputs)
        },

        None => {
            Arity::Constant
        }
    };

    let output = if output_is_a_reference {
        Ty::Reference(output_type)
    } else {
        Ty::Copy(output_type)
    };

    Expression::AnonymousFunction(
        AnonymousFunction {
            declaration_scope : declaration_scope,
            inputs            : inputs,
            output            : output,
            enclosing_scope   : enclosing_scope,
            body              : body
        }
    )
}


#[cfg(test)]
mod tests {
    use super::{
        anonymous_function,
        array,
        intrinsic,
        intrinsic_construct,
        intrinsic_echo,
        intrinsic_empty,
        intrinsic_eval,
        intrinsic_exit,
        intrinsic_isset,
        intrinsic_list,
        intrinsic_operator,
        intrinsic_print,
        intrinsic_unset,
        primary
    };
    use super::super::expression;
    use super::super::super::super::ast::{
        AnonymousFunction,
        Arity,
        Expression,
        Literal,
        Name,
        Parameter,
        Scope,
        Statement,
        Ty,
        Variable
    };
    use super::super::super::super::internal::{
        Error,
        ErrorKind,
        Result
    };

    #[test]
    fn case_array_empty() {
        let input  = b"[ /* foo */ ]";
        let output = Result::Done(&b""[..], Expression::Array(vec![]));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_one_value() {
        let input  = b"['foo']";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::String(b"foo".to_vec()))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_one_pair() {
        let input  = b"[42 => 'foo']";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    Some(Expression::Literal(Literal::Integer(42i64))),
                    Expression::Literal(Literal::String(b"foo".to_vec()))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_many_pairs() {
        let input  = b"['foo', 42 => 'bar', 'baz' => $qux]";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::String(b"foo".to_vec()))
                ),
                (
                    Some(Expression::Literal(Literal::Integer(42i64))),
                    Expression::Literal(Literal::String(b"bar".to_vec()))
                ),
                (
                    Some(Expression::Literal(Literal::String(b"baz".to_vec()))),
                    Expression::Variable(Variable(&b"qux"[..]))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_vector_capacity() {
        if let Result::Done(_, Expression::Array(vector)) = array(b"[1, 2, 3]") {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_array_trailing_comma() {
        let input  = b"[1, 2, 3, /* foo */]";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::Integer(1i64))
                ),
                (
                    None,
                    Expression::Literal(Literal::Integer(2i64))
                ),
                (
                    None,
                    Expression::Literal(Literal::Integer(3i64))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_recursive() {
        let input  = b"['foo', 42 => [3 => 5, 7 => [11 => '13']], 'baz' => $qux]";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::String(b"foo".to_vec()))
                ),
                (
                    Some(Expression::Literal(Literal::Integer(42i64))),
                    Expression::Array(vec![
                        (
                            Some(Expression::Literal(Literal::Integer(3i64))),
                            Expression::Literal(Literal::Integer(5i64))
                        ),
                        (
                            Some(Expression::Literal(Literal::Integer(7i64))),
                            Expression::Array(vec![
                                (
                                    Some(Expression::Literal(Literal::Integer(11i64))),
                                    Expression::Literal(Literal::String(b"13".to_vec()))
                                )
                            ])
                        )
                    ])
                ),
                (
                    Some(Expression::Literal(Literal::String(b"baz".to_vec()))),
                    Expression::Variable(Variable(&b"qux"[..]))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_value_by_reference() {
        let input  = b"[7 => &$foo, 42 => $bar]";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    Some(Expression::Literal(Literal::Integer(7i64))),
                    Expression::Reference(
                        Box::new(Expression::Variable(Variable(&b"foo"[..])))
                    )
                ),
                (
                    Some(Expression::Literal(Literal::Integer(42i64))),
                    Expression::Variable(Variable(&b"bar"[..]))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_array_trailing_commas() {
        let input  = b"[1, 2, 3,,]";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"[1, 2, 3,,]"[..]));

        assert_eq!(array(input), Result::Error(Error::Position(ErrorKind::Alt, &b"[1, 2, 3,,]"[..])));
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_array_empty_trailing_comma() {
        let input  = b"[,]";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"[,]"[..]));

        assert_eq!(array(input), Result::Error(Error::Position(ErrorKind::Alt, &b"[,]"[..])));
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_empty() {
        let input  = b"array ( /* foo */ )";
        let output = Result::Done(&b""[..], Expression::Array(vec![]));

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_one_value() {
        let input  = b"array('foo')";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::String(b"foo".to_vec()))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_one_pair() {
        let input  = b"array(42 => 'foo')";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    Some(Expression::Literal(Literal::Integer(42i64))),
                    Expression::Literal(Literal::String(b"foo".to_vec()))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_many_pairs() {
        let input  = b"array('foo', 42 => 'bar', 'baz' => $qux)";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::String(b"foo".to_vec()))
                ),
                (
                    Some(Expression::Literal(Literal::Integer(42i64))),
                    Expression::Literal(Literal::String(b"bar".to_vec()))
                ),
                (
                    Some(Expression::Literal(Literal::String(b"baz".to_vec()))),
                    Expression::Variable(Variable(&b"qux"[..]))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_trailing_comma() {
        let input  = b"array(1, 2, 3, /* foo */)";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::Integer(1i64))
                ),
                (
                    None,
                    Expression::Literal(Literal::Integer(2i64))
                ),
                (
                    None,
                    Expression::Literal(Literal::Integer(3i64))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_recursive() {
        let input  = b"array('foo', 42 => array(3 => 5, 7 => array(11 => '13')), 'baz' => $qux)";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    None,
                    Expression::Literal(Literal::String(b"foo".to_vec()))
                ),
                (
                    Some(Expression::Literal(Literal::Integer(42i64))),
                    Expression::Array(vec![
                        (
                            Some(Expression::Literal(Literal::Integer(3i64))),
                            Expression::Literal(Literal::Integer(5i64))
                        ),
                        (
                            Some(Expression::Literal(Literal::Integer(7i64))),
                            Expression::Array(vec![
                                (
                                    Some(Expression::Literal(Literal::Integer(11i64))),
                                    Expression::Literal(Literal::String(b"13".to_vec()))
                                )
                            ])
                        )
                    ])
                ),
                (
                    Some(Expression::Literal(Literal::String(b"baz".to_vec()))),
                    Expression::Variable(Variable(&b"qux"[..]))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_array_long_syntax_value_by_reference() {
        let input  = b"array(7 => &$foo, 42 => $bar)";
        let output = Result::Done(
            &b""[..],
            Expression::Array(vec![
                (
                    Some(Expression::Literal(Literal::Integer(7i64))),
                    Expression::Reference(
                        Box::new(Expression::Variable(Variable(&b"foo"[..])))
                    )
                ),
                (
                    Some(Expression::Literal(Literal::Integer(42i64))),
                    Expression::Variable(Variable(&b"bar"[..]))
                )
            ])
        );

        assert_eq!(array(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_variable() {
        let input  = b"$foo";
        let output = Result::Done(&b""[..], Expression::Variable(Variable(&b"foo"[..])));

        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_qualified_name() {
        let input  = b"Foo\\Bar";
        let output = Result::Done(&b""[..], Expression::Name(Name::Qualified(vec![&b"Foo"[..], &b"Bar"[..]])));

        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_literal() {
        let input  = b"'Hello, World!'";
        let output = Result::Done(&b""[..], Expression::Literal(Literal::String(b"Hello, World!".to_vec())));

        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_echo_one_expression() {
        let input  = b"echo /* baz */ 'foobar'";
        let output = Result::Done(
            &b""[..],
            Expression::Echo(vec![
                Expression::Literal(Literal::String(b"foobar".to_vec()))
            ])
        );

        assert_eq!(intrinsic_echo(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_echo_many_expressions() {
        let input  = b"echo /* baz */ 'foobar',\t $bazqux, \n  42";
        let output = Result::Done(
            &b""[..],
            Expression::Echo(vec![
                Expression::Literal(Literal::String(b"foobar".to_vec())),
                Expression::Variable(Variable(&b"bazqux"[..])),
                Expression::Literal(Literal::Integer(42i64))
            ])
        );

        assert_eq!(intrinsic_echo(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_echo_vector_capacity() {
        if let Result::Done(_, Expression::Echo(vector)) = intrinsic_echo(b"echo 'foobar', $bazqux, 42") {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_invalid_intrinsic_echo_expression_missing() {
        let input  = b"echo;";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"echo;"[..]));

        assert_eq!(intrinsic_echo(input), Result::Error(Error::Position(ErrorKind::Alt, &b";"[..])));
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_keyed_one_pattern() {
        let input  = b"list('foo' => $foo)";
        let output = Result::Done(
            &b""[..],
            Expression::List(vec![
                Some((
                    Some(Expression::Literal(Literal::String(b"foo".to_vec()))),
                    Expression::Variable(Variable(&b"foo"[..]))
                ))
            ])
        );

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_keyed_many_patterns() {
        let input  = b"list('foo' => $foo, 'bar' => $bar, 'baz' => $baz)";
        let output = Result::Done(
            &b""[..],
            Expression::List(vec![
                Some((
                    Some(Expression::Literal(Literal::String(b"foo".to_vec()))),
                    Expression::Variable(Variable(&b"foo"[..]))
                )),
                Some((
                    Some(Expression::Literal(Literal::String(b"bar".to_vec()))),
                    Expression::Variable(Variable(&b"bar"[..]))
                )),
                Some((
                    Some(Expression::Literal(Literal::String(b"baz".to_vec()))),
                    Expression::Variable(Variable(&b"baz"[..]))
                ))
            ])
        );

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_keyed_vector_capacity() {
        if let Result::Done(_, Expression::List(vector)) = intrinsic_list(b"list('foo' => $foo, 'bar' => $bar, 'baz' => $baz)") {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_intrinsic_list_keyed_trailing_comma() {
        let input  = b"list('foo' => $foo, 'bar' => $bar,)";
        let output = Result::Done(
            &b""[..],
            Expression::List(vec![
                Some((
                    Some(Expression::Literal(Literal::String(b"foo".to_vec()))),
                    Expression::Variable(Variable(&b"foo"[..]))
                )),
                Some((
                    Some(Expression::Literal(Literal::String(b"bar".to_vec()))),
                    Expression::Variable(Variable(&b"bar"[..]))
                ))
            ])
        );

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_keyed_recursive() {
        let input  = b"list('foo' => list('bar' => $bar), 'baz' => list(, $qux))";
        let output = Result::Done(
            &b""[..],
            Expression::List(vec![
                Some((
                    Some(Expression::Literal(Literal::String(b"foo".to_vec()))),
                    Expression::List(vec![
                        Some((
                            Some(Expression::Literal(Literal::String(b"bar".to_vec()))),
                            Expression::Variable(Variable(&b"bar"[..]))
                        ))
                    ])
                )),
                Some((
                    Some(Expression::Literal(Literal::String(b"baz".to_vec()))),
                    Expression::List(vec![
                        None,
                        Some((
                            None,
                            Expression::Variable(Variable(&b"qux"[..]))
                        ))
                    ])
                ))
            ])
        );

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_unkeyed_one_pattern() {
        let input  = b"list($foo)";
        let output = Result::Done(
            &b""[..],
            Expression::List(vec![
                Some((
                    None,
                    Expression::Variable(Variable(&b"foo"[..]))
                ))
            ])
        );

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_unkeyed_many_patterns() {
        let input  = b"list($foo, $bar, $baz)";
        let output = Result::Done(
            &b""[..],
            Expression::List(vec![
                Some((
                    None,
                    Expression::Variable(Variable(&b"foo"[..]))
                )),
                Some((
                    None,
                    Expression::Variable(Variable(&b"bar"[..]))
                )),
                Some((
                    None,
                    Expression::Variable(Variable(&b"baz"[..]))
                ))
            ])
        );

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_unkeyed_vector_capacity() {
        if let Result::Done(_, Expression::List(vector)) = intrinsic_list(b"list($foo, $bar, $baz)") {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_intrinsic_list_unkeyed_free_patterns() {
        let input  = b"list($foo, , $bar, , , $baz,)";
        let output = Result::Done(
            &b""[..],
            Expression::List(vec![
                Some((
                    None,
                    Expression::Variable(Variable(&b"foo"[..]))
                )),
                None,
                Some((
                    None,
                    Expression::Variable(Variable(&b"bar"[..]))
                )),
                None,
                None,
                Some((
                    None,
                    Expression::Variable(Variable(&b"baz"[..]))
                )),
                None
            ])
        );

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_list_unkeyed_recursive() {
        let input  = b"list($foo, list($bar), list('baz' => $baz))";
        let output = Result::Done(
            &b""[..],
            Expression::List(vec![
                Some((
                    None,
                    Expression::Variable(Variable(&b"foo"[..]))
                )),
                Some((
                    None,
                    Expression::List(vec![
                        Some((
                            None,
                            Expression::Variable(Variable(&b"bar"[..]))
                        ))
                    ])
                )),
                Some((
                    None,
                    Expression::List(vec![
                        Some((
                            Some(Expression::Literal(Literal::String(b"baz".to_vec()))),
                            Expression::Variable(Variable(&b"baz"[..]))
                        ))
                    ])
                ))
            ])
        );

        assert_eq!(intrinsic_list(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_list_mixed_pairs() {
        let input  = b"list('foo' => $foo, $bar)";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"list('foo' => $foo, $bar)"[..]));

        assert_eq!(intrinsic_list(input), Result::Error(Error::Position(ErrorKind::Tag, &b"$bar)"[..])));
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_list_empty() {
        let input  = b"list()";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"list()"[..]));

        assert_eq!(intrinsic_list(input), Result::Error(Error::Position(ErrorKind::MapRes, &b"list()"[..])));
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_list_only_free_patterns() {
        let input  = b"list(,,,)";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"list(,,,)"[..]));

        assert_eq!(intrinsic_list(input), Result::Error(Error::Position(ErrorKind::MapRes, &b"list(,,,)"[..])));
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_unset_one_variable() {
        let input  = b"unset($foo)";
        let output = Result::Done(
            &b""[..],
            Expression::Unset(vec![
                Expression::Variable(Variable(&b"foo"[..]))
            ])
        );

        assert_eq!(intrinsic_unset(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_unset_many_variables() {
        let input  = b"unset($foo, $bar, $baz)";
        let output = Result::Done(
            &b""[..],
            Expression::Unset(vec![
                Expression::Variable(Variable(&b"foo"[..])),
                Expression::Variable(Variable(&b"bar"[..])),
                Expression::Variable(Variable(&b"baz"[..]))
            ])
        );

        assert_eq!(intrinsic_unset(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_unset_vector_capacity() {
        if let Result::Done(_, Expression::Unset(vector)) = intrinsic_unset(b"unset($foo, $bar, $baz)") {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_invalid_intrinsic_unset_zero_variable() {
        let input  = b"unset()";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"unset()"[..]));

        assert_eq!(intrinsic_unset(input), Result::Error(Error::Position(ErrorKind::Alt, &b")"[..])));
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_empty_string() {
        let input  = b"empty('foo')";
        let output = Result::Done(
            &b""[..],
            Expression::Empty(
                Box::new(
                    Expression::Literal(
                        Literal::String(b"foo".to_vec())
                    )
                )
            )
        );

        assert_eq!(intrinsic_empty(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_empty_integer() {
        let input  = b"empty(42)";
        let output = Result::Done(
            &b""[..],
            Expression::Empty(
                Box::new(
                    Expression::Literal(
                        Literal::Integer(42i64)
                    )
                )
            )
        );

        assert_eq!(intrinsic_empty(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_empty_expression_missing() {
        let input  = b"empty()";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"empty()"[..]));

        assert_eq!(intrinsic_empty(input), Result::Error(Error::Position(ErrorKind::Alt, &b")"[..])));
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_eval() {
        let input  = b"eval('1 + 2;')";
        let output = Result::Done(
            &b""[..],
            Expression::Eval(
                Box::new(
                    Expression::Literal(
                        Literal::String(b"1 + 2;".to_vec())
                    )
                )
            )
        );

        assert_eq!(intrinsic_eval(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_eval_expression_missing() {
        let input  = b"eval()";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"eval()"[..]));

        assert_eq!(intrinsic_eval(input), Result::Error(Error::Position(ErrorKind::Alt, &b")"[..])));
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_exit() {
        let input  = b"exit(42)";
        let output = Result::Done(
            &b""[..],
            Expression::Exit(
                Some(
                    Box::new(
                        Expression::Literal(
                            Literal::Integer(42i64)
                        )
                    )
                )
            )
        );

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_exit_with_no_argument() {
        let input  = b"exit 42";
        let output = Result::Done(&b" 42"[..], Expression::Exit(None));

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_exit_with_a_variable() {
        let input  = b"exit($foo)";
        let output = Result::Done(
            &b""[..],
            Expression::Exit(
                Some(
                    Box::new(
                        Expression::Variable(
                            Variable(&b"foo"[..])
                        )
                    )
                )
            )
        );

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_exit_with_reserved_code_255() {
        let input  = b"exit(255)";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"exit(255)"[..]));

        assert_eq!(intrinsic_exit(input), Result::Error(Error::Position(ErrorKind::MapRes, &b"exit(255)"[..])));
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_exit_with_out_of_range_code() {
        let input  = b"exit(256)";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"exit(256)"[..]));

        assert_eq!(intrinsic_exit(input), Result::Error(Error::Position(ErrorKind::MapRes, &b"exit(256)"[..])));
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_die() {
        let input  = b"die(42)";
        let output = Result::Done(
            &b""[..],
            Expression::Exit(
                Some(
                    Box::new(
                        Expression::Literal(
                            Literal::Integer(42i64)
                        )
                    )
                )
            )
        );

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_die_with_no_parenthesis() {
        let input  = b"die 42";
        let output = Result::Done(&b" 42"[..], Expression::Exit(None));

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_die_with_a_variable() {
        let input  = b"die($foo)";
        let output = Result::Done(
            &b""[..],
            Expression::Exit(
                Some(
                    Box::new(
                        Expression::Variable(
                            Variable(&b"foo"[..])
                        )
                    )
                )
            )
        );

        assert_eq!(intrinsic_exit(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_die_with_reserved_code_255() {
        let input  = b"die(255)";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"die(255)"[..]));

        assert_eq!(intrinsic_exit(input), Result::Error(Error::Position(ErrorKind::MapRes, &b"die(255)"[..])));
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_die_with_out_of_range_code() {
        let input  = b"die(256)";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"die(256)"[..]));

        assert_eq!(intrinsic_exit(input), Result::Error(Error::Position(ErrorKind::MapRes, &b"die(256)"[..])));
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_isset_one_variable() {
        let input  = b"isset($foo)";
        let output = Result::Done(
            &b""[..],
            Expression::Isset(vec![
                Expression::Variable(Variable(&b"foo"[..]))
            ])
        );

        assert_eq!(intrinsic_isset(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_isset_many_variables() {
        let input  = b"isset($foo, $bar, $baz)";
        let output = Result::Done(
            &b""[..],
            Expression::Isset(vec![
                Expression::Variable(Variable(&b"foo"[..])),
                Expression::Variable(Variable(&b"bar"[..])),
                Expression::Variable(Variable(&b"baz"[..]))
            ])
        );

        assert_eq!(intrinsic_isset(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_isset_vector_capacity() {
        if let Result::Done(_, Expression::Isset(vector)) = intrinsic_isset(b"isset($foo, $bar, $baz)") {
            assert_eq!(vector.capacity(), vector.len());
            assert_eq!(vector.len(), 3);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn case_invalid_intrinsic_isset_zero_variable() {
        let input  = b"isset()";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"isset()"[..]));

        assert_eq!(intrinsic_isset(input), Result::Error(Error::Position(ErrorKind::Alt, &b")"[..])));
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_print() {
        let input  = b"print /* baz */ 'foobar'";
        let output = Result::Done(
            &b""[..],
            Expression::Print(
                Box::new(
                    Expression::Literal(Literal::String(b"foobar".to_vec()))
                )
            )
        );

        assert_eq!(intrinsic_print(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_print_expression_missing() {
        let input  = b"print;";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"print;"[..]));

        assert_eq!(intrinsic_print(input), Result::Error(Error::Position(ErrorKind::Alt, &b";"[..])));
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_grouped_by_parenthesis() {
        let input  = b"print (((('foobar'))))";
        let output = intrinsic_print(b"print 'foobar'");

        assert_eq!(intrinsic_print(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function() {
        let input  = b"function (I $x, J &$y): O use ($z) { return; }";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Finite(vec![
                        Parameter {
                            ty   : Ty::Copy(Some(Name::Unqualified(&b"I"[..]))),
                            name : Variable(&b"x"[..]),
                            value: None
                        },
                        Parameter {
                            ty   : Ty::Reference(Some(Name::Unqualified(&b"J"[..]))),
                            name : Variable(&b"y"[..]),
                            value: None
                        }
                    ]),
                    output         : Ty::Copy(Some(Name::Unqualified(&b"O"[..]))),
                    enclosing_scope: Some(vec![Expression::Variable(Variable(&b"z"[..]))]),
                    body           : vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_zero() {
        let input  = b"function () {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Constant,
                    output           : Ty::Copy(None),
                    enclosing_scope  : None,
                    body             : vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_one_by_copy() {
        let input  = b"function ($x) {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Finite(vec![
                        Parameter {
                            ty   : Ty::Copy(None),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output         : Ty::Copy(None),
                    enclosing_scope: None,
                    body           : vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_one_by_reference() {
        let input  = b"function (&$x) {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Finite(vec![
                        Parameter {
                            ty   : Ty::Reference(None),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output         : Ty::Copy(None),
                    enclosing_scope: None,
                    body           : vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_one_with_a_copy_type() {
        let input  = b"function (A\\B\\C $x) {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Finite(vec![
                        Parameter {
                            ty   : Ty::Copy(Some(Name::Qualified(vec![&b"A"[..], &b"B"[..], &b"C"[..]]))),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output           : Ty::Copy(None),
                    enclosing_scope  : None,
                    body             : vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_one_with_a_reference_type() {
        let input  = b"function (int &$x) {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Finite(vec![
                        Parameter {
                            ty   : Ty::Reference(Some(Name::Unqualified(&b"int"[..]))),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output         : Ty::Copy(None),
                    enclosing_scope: None,
                    body           : vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_arity_many() {
        let input  = b"function ($a, I\\J $b, int &$c, \\K $d) {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Finite(vec![
                        Parameter {
                            ty   : Ty::Copy(None),
                            name : Variable(&b"a"[..]),
                            value: None
                        },
                        Parameter {
                            ty   : Ty::Copy(Some(Name::Qualified(vec![&b"I"[..], &b"J"[..]]))),
                            name : Variable(&b"b"[..]),
                            value: None
                        },
                        Parameter {
                            ty   : Ty::Reference(Some(Name::Unqualified(&b"int"[..]))),
                            name : Variable(&b"c"[..]),
                            value: None
                        },
                        Parameter {
                            ty   : Ty::Copy(Some(Name::FullyQualified(vec![&b"K"[..]]))),
                            name : Variable(&b"d"[..]),
                            value: None
                        }
                    ]),
                    output         : Ty::Copy(None),
                    enclosing_scope: None,
                    body           : vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_output_by_copy() {
        let input  = b"function (): \\O {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Constant,
                    output           : Ty::Copy(Some(Name::FullyQualified(vec![&b"O"[..]]))),
                    enclosing_scope  : None,
                    body             : vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_output_by_reference() {
        let input  = b"function &(): int {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Constant,
                    output           : Ty::Reference(Some(Name::Unqualified(&b"int"[..]))),
                    enclosing_scope  : None,
                    body             : vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_empty_enclosing_scope() {
        let input  = b"function () use () {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Constant,
                    output           : Ty::Copy(None),
                    enclosing_scope  : Some(vec![]),
                    body             : vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_one_enclosed_variable_by_copy() {
        let input  = b"function () use ($x) {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Constant,
                    output           : Ty::Copy(None),
                    enclosing_scope  : Some(vec![
                        Expression::Variable(Variable(&b"x"[..]))
                    ]),
                    body: vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_one_enclosed_variable_by_reference() {
        let input  = b"function () use (&$x) {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Constant,
                    output           : Ty::Copy(None),
                    enclosing_scope  : Some(vec![
                        Expression::Reference(
                            Box::new(
                                Expression::Variable(Variable(&b"x"[..]))
                            )
                        )
                    ]),
                    body: vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_many_enclosed_variables() {
        let input  = b"function () use ($x, &$y, $z) {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Dynamic,
                    inputs           : Arity::Constant,
                    output           : Ty::Copy(None),
                    enclosing_scope  : Some(vec![
                        Expression::Variable(Variable(&b"x"[..])),
                        Expression::Reference(
                            Box::new(
                                Expression::Variable(Variable(&b"y"[..]))
                            )
                        ),
                        Expression::Variable(Variable(&b"z"[..]))
                    ]),
                    body: vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_anonymous_function_static_scope() {
        let input  = b"static function () {}";
        let output = Result::Done(
            &b""[..],
            Expression::AnonymousFunction(
                AnonymousFunction {
                    declaration_scope: Scope::Static,
                    inputs           : Arity::Constant,
                    output           : Ty::Copy(None),
                    enclosing_scope  : None,
                    body             : vec![Statement::Return]
                }
            )
        );

        assert_eq!(anonymous_function(input), output);
        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }
}
