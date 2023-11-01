/******************************************************************************/
/*                                                                            */
/* SPDX-License-Identifier: MIT                                               */
/* Copyright (c) [2023] Serokell <hi@serokell.io>                             */
/*                                                                            */
/******************************************************************************/

mod context;
mod expectation;

use crate::ast::*;
use crate::context::*;
use crate::interpreter::*;
use crate::parser::spanned_lexer;
use crate::stack::*;
use crate::syntax::tztTestEntitiesParser;
use crate::typechecker::*;
use crate::tzt::context::*;
use crate::tzt::expectation::*;

pub type TestStack = Vec<(Type, TypedValue)>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TztTestError {
    StackMismatch(
        (FailingTypeStack, Stack<Value>),
        (FailingTypeStack, Stack<Value>),
    ),
    UnexpectedError(TestError),
    UnexpectedSuccess(IStack),
    ExpectedDifferentError(ErrorExpectation, TestError),
}

/// Represent one Tzt test. The output attribute is a Result to include
/// expectation of failure.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TztTest {
    pub code: ParsedInstructionBlock,
    pub input: TestStack,
    pub output: TestExpectation,
    pub amount: Option<i64>,
}

fn typecheck_stack(stk: Vec<(Type, Value)>) -> Result<Vec<(Type, TypedValue)>, TcError> {
    stk.into_iter()
        .map(|(t, v)| {
            let tc_val = typecheck_value(&mut Default::default(), &t, v)?;
            Ok((t, tc_val))
        })
        .collect()
}

#[allow(dead_code)]
pub fn parse_tzt_test(src: &str) -> Result<TztTest, Box<dyn Error + '_>> {
    tztTestEntitiesParser::new()
        .parse(spanned_lexer(src))?
        .try_into()
}

// Check if the option argument value is none, and raise an error if it is not.
// If it is none, then fill it with the provided value.
fn set_tzt_field<T>(field_name: &str, t: &mut Option<T>, v: T) -> Result<(), String> {
    match t {
        Some(_) => Err(format!("Duplicate field '{}' in test", field_name)),
        None => {
            *t = Some(v);
            Ok(())
        }
    }
}

use std::error::Error;
impl TryFrom<Vec<TztEntity>> for TztTest {
    type Error = Box<dyn Error>;
    fn try_from(tzt: Vec<TztEntity>) -> Result<Self, Self::Error> {
        use ErrorExpectation::*;
        use TestExpectation::*;
        use TztEntity::*;
        use TztOutput::*;
        let mut m_code: Option<ParsedInstructionBlock> = None;
        let mut m_input: Option<TestStack> = None;
        let mut m_output: Option<TestExpectation> = None;
        let mut m_amount: Option<i64> = None;

        for e in tzt {
            match e {
                Code(ib) => set_tzt_field("code", &mut m_code, ib)?,
                Input(stk) => set_tzt_field("input", &mut m_input, typecheck_stack(stk)?)?,
                Output(tzt_output) => set_tzt_field(
                    "output",
                    &mut m_output,
                    match tzt_output {
                        Success(stk) => {
                            typecheck_stack(stk.clone())?;
                            ExpectSuccess(stk)
                        }
                        Fail(v) => ExpectError(InterpreterError(
                            InterpreterErrorExpectation::FailedWith(v),
                        )),
                        TztOutput::MutezOverflow(v1, v2) => ExpectError(InterpreterError(
                            InterpreterErrorExpectation::MutezOverflow(v1, v2),
                        )),
                        TztOutput::GeneralOverflow(_, _) => {
                            todo!("General overflow is not implemented!")
                        }
                    },
                )?,
                Amount(m) => set_tzt_field("amount", &mut m_amount, m)?,
            }
        }

        Ok(TztTest {
            code: m_code.ok_or("code section not found in test")?,
            input: m_input.ok_or("input section not found in test")?,
            output: m_output.ok_or("output section not found in test")?,
            amount: m_amount,
        })
    }
}

/// This represents possibilities in which the execution of
/// the code in a test can fail.
#[derive(Debug, PartialEq, Eq, Clone, thiserror::Error)]
pub enum TestError {
    #[error(transparent)]
    TypecheckerError(#[from] TcError),
    #[error(transparent)]
    InterpreterError(#[from] InterpretError),
}

/// This represents the outcome that we expect from interpreting
/// the code in a test.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TestExpectation {
    ExpectSuccess(Vec<(Type, Value)>),
    ExpectError(ErrorExpectation),
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ErrorExpectation {
    TypecheckerError(TcError),
    InterpreterError(InterpreterErrorExpectation),
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum InterpreterErrorExpectation {
    GeneralOverflow(i128, i128),
    MutezOverflow(i64, i64),
    FailedWith(Value),
}

/// Helper type for use during parsing, represent a single
/// line from the test file.
pub enum TztEntity {
    Code(ParsedInstructionBlock),
    Input(Vec<(Type, Value)>),
    Output(TztOutput),
    Amount(i64),
}

/// Possible values for the "output" field in a Tzt test
pub enum TztOutput {
    Success(Vec<(Type, Value)>),
    Fail(Value),
    MutezOverflow(i64, i64),
    GeneralOverflow(i128, i128),
}

fn execute_tzt_test_code(
    code: ParsedInstructionBlock,
    ctx: &mut Ctx,
    input: Vec<(Type, TypedValue)>,
) -> Result<(FailingTypeStack, IStack), TestError> {
    // Build initial stacks (type and value) for running the test from the test input
    // stack.
    let (typs, vals): (Vec<Type>, Vec<TypedValue>) = input.into_iter().unzip();

    let mut t_stack: FailingTypeStack = FailingTypeStack::Ok(TopIsFirst::from(typs).0);

    // Run the code and save the status of the
    // final result as a Result<(), TestError>.
    //
    // This value along with the test expectation
    // from the test file will be used to decide if
    // the test was a success or a fail.
    let typechecked_code = typecheck(code, ctx, &mut t_stack)?;
    let mut i_stack: IStack = TopIsFirst::from(vals).0;
    interpret(&typechecked_code, ctx, &mut i_stack)?;
    Ok((t_stack, i_stack))
}

#[allow(dead_code)]
pub fn run_tzt_test(test: TztTest) -> Result<(), TztTestError> {
    // Here we compare the outcome of the interpreting with the
    // expectation from the test, and declare the result of the test
    // accordingly.
    let mut ctx = construct_context(&test);
    let execution_result = execute_tzt_test_code(test.code, &mut ctx, test.input);
    check_expectation(&mut ctx, test.output, execution_result)
}