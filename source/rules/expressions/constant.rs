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

//! Group of constant expression rules.
//!
//! The list of all constant expressions is provided by the PHP Language
//! Specification in the [Grammar chapter, Expressions
//! section](https://github.com/php/php-langspec/blob/master/spec/19-grammar.md#constant-expressions).

use super::primaries::array;
use super::super::literals::literal;
use super::super::super::ast::{
    Expression,
    Literal
};

named_attr!(
    #[doc="
        Recognize all kind of constant expressions.
    "],
    pub constant_expression<Expression>,
    alt!(
        literal => { literal_mapper }
      | array
    )
);

#[inline(always)]
fn literal_mapper<'a>(literal: Literal) -> Expression<'a> {
    Expression::Literal(literal)
}


/*
#[cfg(test)]
mod tests {
    use super::constant;
    use super::super::expression;
    use super::super::super::super::ast::{
        Expression,
        Literal
    };
    use super::super::super::super::internal::Result;
}
*/
