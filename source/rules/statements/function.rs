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

//! Group of function rules.
//!
//! The list of all function rules is provided by the PHP Language
//! Specification in the [Chapter chapter, Function Definition
//! section](https://github.com/php/php-langspec/blob/master/spec/19-grammar.md#function-definition).

use std::result::Result as StdResult;
use super::compound_statement;
use super::super::tokens::{
    name,
    qualified_name,
    variable
};
use super::super::super::ast::{
    Arity,
    Function,
    Name,
    Parameter,
    Statement,
    Ty,
    Variable
};
use super::super::super::internal::{
    Error,
    ErrorKind
};
use super::super::super::tokens;

/// Function errors.
pub enum FunctionError {
    /// A variadic function has a `...parameter` at an invalid
    /// position. It must be the latest one.
    InvalidVariadicParameterPosition
}


named_attr!(
    #[doc="
        Recognize a function.

        # Examples

        A function with 3 inputs, aka parameters:

        1. `$x`, untyped and passed by copy,
        2. `$y`, typed with a fully-qualified name, and passed by
           an implicit reference (this is a copy type, but the type
           is an object, so this is always a reference),
        3. `$z`, typed with a primite type, and passed by reference.

        The output is also typed with a unqualified name, and
        explicitly passed by reference.

        The arity of this function is finite.

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{
            Arity,
            Function,
            Name,
            Parameter,
            Statement,
            Ty,
            Variable
        };
        use tagua_parser::rules::statements::function::function;

        # fn main() {
        assert_eq!(
            function(b\"function &f($x, \\\\I\\\\J $y, int &$z): O { return; }\"),
            Result::Done(
                &b\"\"[..],
                Statement::Function(
                    Function {
                        name  : &b\"f\"[..],
                        inputs: Arity::Finite(vec![
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
                        output: Ty::Reference(Some(Name::Unqualified(&b\"O\"[..]))),
                        body  : vec![Statement::Return]
                    }
                )
            )
        );
        # }
        ```

        This function has an infinite arity. This is also called a
        variadic function. The last parameter receives all extra
        arguments.

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{
            Arity,
            Function,
            Name,
            Parameter,
            Statement,
            Ty,
            Variable
        };
        use tagua_parser::rules::statements::function::function;

        # fn main() {
        assert_eq!(
            function(b\"function f($x, int ...$y) { return; }\"),
            Result::Done(
                &b\"\"[..],
                Statement::Function(
                    Function {
                        name  : &b\"f\"[..],
                        inputs: Arity::Infinite(vec![
                            Parameter {
                                ty   : Ty::Copy(None),
                                name : Variable(&b\"x\"[..]),
                                value: None
                            },
                            Parameter {
                                ty   : Ty::Copy(Some(Name::Unqualified(&b\"int\"[..]))),
                                name : Variable(&b\"y\"[..]),
                                value: None
                            }
                        ]),
                        output: Ty::Copy(None),
                        body  : vec![Statement::Return]
                    }
                )
            )
        );
        # }
        ```
    "],
    pub function<Statement>,
    do_parse!(
        first!(keyword!(tokens::FUNCTION)) >>
        output_is_a_reference: opt!(first!(tag!(tokens::REFERENCE))) >>
        name: first!(name) >>
        inputs: first!(parameters) >>
        output_type: opt!(
            preceded!(
                first!(tag!(tokens::FUNCTION_OUTPUT)),
                first!(qualified_name)
            )
        ) >>
        body: first!(compound_statement) >>
        (
            into_function(
                name,
                inputs,
                output_is_a_reference.is_some(),
                output_type,
                body
            )
        )
    )
);

named_attr!(
    #[doc="
        Recognize a list of function parameters.

        # Examples

        ```
        use tagua_parser::Result;
        use tagua_parser::ast::{
            Arity,
            Name,
            Parameter,
            Ty,
            Variable
        };
        use tagua_parser::rules::statements::function::parameters;

        # fn main() {
        assert_eq!(
            parameters(b\"($x, \\\\I\\\\J $y, int &$z)\"),
            Result::Done(
                &b\"\"[..],
                Arity::Finite(vec![
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
                ])
            )
        );
        # }
        ```
    "],
    pub parameters<Arity>,
    map_res!(
        terminated!(
            preceded!(
                tag!(tokens::LEFT_PARENTHESIS),
                opt!(
                    do_parse!(
                        accumulator: map_res!(
                            first!(parameter),
                            into_vector_mapper
                        ) >>
                        result: fold_into_vector_many0!(
                            preceded!(
                                first!(tag!(tokens::COMMA)),
                                first!(parameter)
                            ),
                            accumulator
                        ) >>
                        (result)
                    )
                )
            ),
            first!(tag!(tokens::RIGHT_PARENTHESIS))
        ),
        parameters_mapper
    )
);

#[inline(always)]
fn parameters_mapper<'a>(pairs: Option<Vec<(Parameter<'a>, bool)>>) -> StdResult<Arity, Error<ErrorKind>> {
    let mut pairs = match pairs {
        Some(pairs) => {
            pairs
        },

        None => {
            return Ok(Arity::Constant);
        }
    };

    let last_pair      = pairs.pop();
    let mut parameters = Vec::new();

    for (parameter, is_variadic) in pairs {
        if is_variadic {
            return Err(Error::Code(ErrorKind::Custom(FunctionError::InvalidVariadicParameterPosition as u32)));
        }

        parameters.push(parameter);
    }

    match last_pair {
        Some((last_parameter, is_variadic)) => {
            parameters.push(last_parameter);

            if is_variadic {
                Ok(Arity::Infinite(parameters))
            } else {
                Ok(Arity::Finite(parameters))
            }
        },

        None => {
            Ok(Arity::Constant)
        }
    }
}

named!(
    parameter< (Parameter, bool) >,
    do_parse!(
        ty: opt!(qualified_name) >>
        is_a_reference: opt!(first!(tag!(tokens::REFERENCE))) >>
        is_variadic: opt!(first!(tag!(tokens::ELLIPSIS))) >>
        name: first!(variable) >>
        (
            into_parameter(
                ty,
                is_a_reference.is_some(),
                is_variadic.is_some(),
                name
            )
        )
    )
);

#[inline(always)]
fn into_vector_mapper<T>(item: T) -> StdResult<Vec<T>, ()> {
    Ok(vec![item])
}

#[inline(always)]
fn into_parameter<'a>(
    ty            : Option<Name<'a>>,
    is_a_reference: bool,
    is_variadic   : bool,
    name          : Variable<'a>
) -> (Parameter<'a>, bool) {
    (
        Parameter {
            ty   : if is_a_reference { Ty::Reference(ty) } else { Ty::Copy(ty) },
            name : name,
            value: None
        },
        is_variadic
    )
}

#[inline(always)]
fn into_function<'a>(
    name                 : &'a [u8],
    inputs               : Arity<'a>,
    output_is_a_reference: bool,
    output_type          : Option<Name<'a>>,
    body                 : Vec<Statement<'a>>
) -> Statement<'a> {
    let output = if output_is_a_reference {
        Ty::Reference(output_type)
    } else {
        Ty::Copy(output_type)
    };

    Statement::Function(
        Function {
            name  : name,
            inputs: inputs,
            output: output,
            body  : body
        }
    )
}


#[cfg(test)]
mod tests {
    use super::{
        function,
        parameters
    };
    use super::super::statement;
    use super::super::super::super::ast::{
        Arity,
        Function,
        Name,
        Parameter,
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
    fn case_function() {
        let input  = b"function f(I $x, J &$y): O { return; }";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Finite(vec![
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
                    output: Ty::Copy(Some(Name::Unqualified(&b"O"[..]))),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_function_arity_zero() {
        let input  = b"function f() {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Constant,
                    output: Ty::Copy(None),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_function_arity_one_by_copy() {
        let input  = b"function f($x) {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Finite(vec![
                        Parameter {
                            ty   : Ty::Copy(None),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output: Ty::Copy(None),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_function_arity_one_by_reference() {
        let input  = b"function f(&$x) {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Finite(vec![
                        Parameter {
                            ty   : Ty::Reference(None),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output: Ty::Copy(None),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_function_arity_one_with_a_copy_type() {
        let input  = b"function f(A\\B\\C $x) {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Finite(vec![
                        Parameter {
                            ty   : Ty::Copy(Some(Name::Qualified(vec![&b"A"[..], &b"B"[..], &b"C"[..]]))),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output: Ty::Copy(None),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_function_arity_one_with_a_reference_type() {
        let input  = b"function f(int &$x) {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Finite(vec![
                        Parameter {
                            ty   : Ty::Reference(Some(Name::Unqualified(&b"int"[..]))),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output: Ty::Copy(None),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_function_arity_many() {
        let input  = b"function f($a, I\\J $b, int &$c, \\K $d) {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Finite(vec![
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
                    output: Ty::Copy(None),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_variadic_function_arity_one_by_copy() {
        let input  = b"function f(...$x) {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Infinite(vec![
                        Parameter {
                            ty   : Ty::Copy(None),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output: Ty::Copy(None),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_variadic_function_arity_one_by_reference() {
        let input  = b"function f(&...$x) {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Infinite(vec![
                        Parameter {
                            ty   : Ty::Reference(None),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output: Ty::Copy(None),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_variadic_function_arity_one_with_a_copy_type() {
        let input  = b"function f(A\\B\\C ...$x) {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Infinite(vec![
                        Parameter {
                            ty   : Ty::Copy(Some(Name::Qualified(vec![&b"A"[..], &b"B"[..], &b"C"[..]]))),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output: Ty::Copy(None),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_variadic_function_arity_one_with_a_reference_type() {
        let input  = b"function f(int &...$x) {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Infinite(vec![
                        Parameter {
                            ty   : Ty::Reference(Some(Name::Unqualified(&b"int"[..]))),
                            name : Variable(&b"x"[..]),
                            value: None
                        }
                    ]),
                    output: Ty::Copy(None),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_variadic_function_arity_many() {
        let input  = b"function f($a, I\\J $b, int &...$c) {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Infinite(vec![
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
                        }
                    ]),
                    output: Ty::Copy(None),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_invalid_variadic_function_parameter_position() {
        let input  = b"function f(...$x, $y) {}";

        assert_eq!(function(input),  Result::Error(Error::Position(ErrorKind::MapRes, &b"(...$x, $y) {}"[..])));
        assert_eq!(statement(input), Result::Error(Error::Position(ErrorKind::Alt, &b"function f(...$x, $y) {}"[..])));
    }

    #[test]
    fn case_function_output_by_copy() {
        let input  = b"function f(): \\O {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Constant,
                    output: Ty::Copy(Some(Name::FullyQualified(vec![&b"O"[..]]))),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_function_output_by_reference() {
        let input  = b"function &f(): int {}";
        let output = Result::Done(
            &b""[..],
            Statement::Function(
                Function {
                    name  : &b"f"[..],
                    inputs: Arity::Constant,
                    output: Ty::Reference(Some(Name::Unqualified(&b"int"[..]))),
                    body  : vec![Statement::Return]
                }
            )
        );

        assert_eq!(function(input), output);
        assert_eq!(statement(input), output);
    }

    #[test]
    fn case_parameters_one_by_copy() {
        let input  = b"($x)";
        let output = Result::Done(
            &b""[..],
            Arity::Finite(vec![
                Parameter {
                    ty   : Ty::Copy(None),
                    name : Variable(&b"x"[..]),
                    value: None
                }
            ])
        );

        assert_eq!(parameters(input), output);
    }

    #[test]
    fn case_parameters_one_by_reference() {
        let input  = b"(&$x)";
        let output = Result::Done(
            &b""[..],
            Arity::Finite(vec![
                Parameter {
                    ty   : Ty::Reference(None),
                    name : Variable(&b"x"[..]),
                    value: None
                }
            ])
        );

        assert_eq!(parameters(input), output);
    }

    #[test]
    fn case_parameters_one_with_a_copy_type() {
        let input  = b"(A\\B\\C $x)";
        let output = Result::Done(
            &b""[..],
            Arity::Finite(vec![
                Parameter {
                    ty   : Ty::Copy(Some(Name::Qualified(vec![&b"A"[..], &b"B"[..], &b"C"[..]]))),
                    name : Variable(&b"x"[..]),
                    value: None
                }
            ])
        );

        assert_eq!(parameters(input), output);
    }

    #[test]
    fn case_parameters_one_with_a_reference_type() {
        let input  = b"(int &$x)";
        let output = Result::Done(
            &b""[..],
            Arity::Finite(vec![
                Parameter {
                    ty   : Ty::Reference(Some(Name::Unqualified(&b"int"[..]))),
                    name : Variable(&b"x"[..]),
                    value: None
                }
            ])
        );

        assert_eq!(parameters(input), output);
    }

    #[test]
    fn case_parameters_one_variadic() {
        let input  = b"(...$x)";
        let output = Result::Done(
            &b""[..],
            Arity::Infinite(vec![
                Parameter {
                    ty   : Ty::Copy(None),
                    name : Variable(&b"x"[..]),
                    value: None
                }
            ])
        );

        assert_eq!(parameters(input), output);
    }

    #[test]
    fn case_parameters_one_variadic_with_a_reference_type() {
        let input  = b"(I &...$x)";
        let output = Result::Done(
            &b""[..],
            Arity::Infinite(vec![
                Parameter {
                    ty   : Ty::Reference(Some(Name::Unqualified(&b"I"[..]))),
                    name : Variable(&b"x"[..]),
                    value: None
                }
            ])
        );

        assert_eq!(parameters(input), output);
    }

    #[test]
    fn case_parameters_many() {
        let input  = b"(&$x, int $y, I\\J $z)";
        let output = Result::Done(
            &b""[..],
            Arity::Finite(vec![
                Parameter {
                    ty   : Ty::Reference(None),
                    name : Variable(&b"x"[..]),
                    value: None
                },
                Parameter {
                    ty   : Ty::Copy(Some(Name::Unqualified(&b"int"[..]))),
                    name : Variable(&b"y"[..]),
                    value: None
                },
                Parameter {
                    ty   : Ty::Copy(Some(Name::Qualified(vec![&b"I"[..], &b"J"[..]]))),
                    name : Variable(&b"z"[..]),
                    value: None
                }
            ])
        );

        assert_eq!(parameters(input), output);
    }

    #[test]
    fn case_parameters_many_variadic() {
        let input  = b"(&$x, int $y, I\\J ...$z)";
        let output = Result::Done(
            &b""[..],
            Arity::Infinite(vec![
                Parameter {
                    ty   : Ty::Reference(None),
                    name : Variable(&b"x"[..]),
                    value: None
                },
                Parameter {
                    ty   : Ty::Copy(Some(Name::Unqualified(&b"int"[..]))),
                    name : Variable(&b"y"[..]),
                    value: None
                },
                Parameter {
                    ty   : Ty::Copy(Some(Name::Qualified(vec![&b"I"[..], &b"J"[..]]))),
                    name : Variable(&b"z"[..]),
                    value: None
                }
            ])
        );

        assert_eq!(parameters(input), output);
    }

    #[test]
    fn case_invalid_parameters_variadic_position() {
        let input  = b"(...$x, $y)";

        assert_eq!(parameters(input), Result::Error(Error::Position(ErrorKind::MapRes, &b"(...$x, $y)"[..])));
    }
}
