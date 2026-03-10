import unittest

from axiom.api import parse_program
from axiom.ast import BlockStmt, FunctionDefStmt, IntLit, Param, Program, ReturnStmt, Span, TypeRef
from axiom.errors import AxiomCompileError
from axiom.semantic_plan import build_semantic_plan


class SemanticPlanTests(unittest.TestCase):
    def test_semantic_plan_qualifies_nested_functions_and_scope_resolution(self) -> None:
        program = parse_program(
            """
fn outer(): int {
  fn inner(): int {
    fn deep(): int {
      return 1
    }
    return deep()
  }
  return inner()
}
"""
        )
        plan = build_semantic_plan(
            program,
            error_factory=lambda message, span: AxiomCompileError(message, span),
        )

        self.assertEqual(
            plan.function_decl_order,
            ["outer", "outer.inner", "outer.inner.deep"],
        )
        self.assertEqual(
            plan.resolve_function("inner", plan.scope_stack_for("outer")),
            "outer.inner",
        )
        self.assertEqual(
            plan.resolve_function("deep", plan.scope_stack_for("outer.inner")),
            "outer.inner.deep",
        )
        self.assertEqual(
            plan.resolve_function("outer.inner", plan.scope_stack_for("outer")),
            "outer.inner",
        )

    def test_semantic_plan_rejects_reserved_param_names(self) -> None:
        program = Program(
            [
                FunctionDefStmt(
                    name="f",
                    params=[
                        Param(
                            name="host",
                            type_ref=TypeRef(name="int", span=Span(0, 4)),
                            span=Span(0, 4),
                        )
                    ],
                    return_type=TypeRef(name="int", span=Span(0, 4)),
                    body=BlockStmt(
                        stmts=[ReturnStmt(expr=IntLit(1, Span(0, 1)), span=Span(0, 1))],
                        span=Span(0, 2),
                    ),
                    span=Span(0, 5),
                )
            ]
        )

        with self.assertRaises(AxiomCompileError):
            build_semantic_plan(
                program,
                error_factory=lambda message, span: AxiomCompileError(message, span),
            )


if __name__ == "__main__":
    unittest.main()
