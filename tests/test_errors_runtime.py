import io
from unittest.mock import patch
import unittest

from axiom.api import compile_to_bytecode, parse_program
from axiom.ast import (
    AssignStmt,
    BlockStmt,
    FunctionDefStmt,
    IntLit,
    LetStmt,
    Param,
    Program,
    ReturnStmt,
    Span,
    TypeRef,
    VarRef,
)
from axiom.bytecode import Bytecode, FunctionMeta, Instr, Op, VERSION_MINOR
from axiom.errors import AxiomCompileError, AxiomParseError, AxiomRuntimeError
from axiom.host import (
    MAX_POW_BASE,
    MAX_POW_EXPONENT,
    host_contract_metadata,
    register_host_builtin,
    reset_host_builtins,
)
from axiom.interpreter import Interpreter
from axiom.vm import Vm


class RuntimeErrorTests(unittest.TestCase):
    def test_runtime_host_version(self) -> None:
        out = io.StringIO()
        Interpreter().run(parse_program("print host.version()\n"), out)
        self.assertEqual(out.getvalue(), f"{VERSION_MINOR}\n")

    def test_runtime_host_abs(self) -> None:
        out = io.StringIO()
        Interpreter().run(parse_program("print host.abs(-12)\n"), out)
        self.assertEqual(out.getvalue(), "12\n")

    def test_runtime_host_math_pow_allows_capped_exponent(self) -> None:
        out = io.StringIO()
        Interpreter().run(parse_program(f"print host.math.pow(2, {MAX_POW_EXPONENT})\n"), out)
        self.assertEqual(out.getvalue(), f"{2 ** MAX_POW_EXPONENT}\n")

    def test_runtime_host_math_pow_rejects_exponent_above_limit(self) -> None:
        with self.assertRaises(AxiomRuntimeError) as cm:
            Interpreter().run(
                parse_program(f"print host.math.pow(2, {MAX_POW_EXPONENT + 1})\n"),
                io.StringIO(),
            )
        self.assertIn(
            f"exponent {MAX_POW_EXPONENT + 1} exceeds limit {MAX_POW_EXPONENT}",
            str(cm.exception),
        )

    def test_runtime_host_math_pow_rejects_base_above_limit(self) -> None:
        base = MAX_POW_BASE + 1
        with self.assertRaises(AxiomRuntimeError) as cm:
            Interpreter().run(parse_program(f"print host.math.pow({base}, 2)\n"), io.StringIO())
        self.assertIn(
            f"base {base} exceeds absolute limit {MAX_POW_BASE}",
            str(cm.exception),
        )

    def test_vm_host_version(self) -> None:
        bc = compile_to_bytecode("print host.version()\n")
        out = io.StringIO()
        Vm(locals_count=bc.locals_count).run(bc, out)
        self.assertEqual(out.getvalue(), f"{VERSION_MINOR}\n")

    def test_vm_host_abs(self) -> None:
        bc = compile_to_bytecode("print host.abs(-12)\n")
        out = io.StringIO()
        Vm(locals_count=bc.locals_count).run(bc, out)
        self.assertEqual(out.getvalue(), "12\n")

    def test_vm_host_math_pow_allows_capped_exponent(self) -> None:
        bc = compile_to_bytecode(f"print host.math.pow(2, {MAX_POW_EXPONENT})\n")
        out = io.StringIO()
        Vm(locals_count=bc.locals_count).run(bc, out)
        self.assertEqual(out.getvalue(), f"{2 ** MAX_POW_EXPONENT}\n")

    def test_vm_host_math_pow_rejects_exponent_above_limit(self) -> None:
        bc = compile_to_bytecode(f"print host.math.pow(2, {MAX_POW_EXPONENT + 1})\n")
        with self.assertRaises(AxiomRuntimeError) as cm:
            Vm(locals_count=bc.locals_count).run(bc, io.StringIO())
        self.assertIn(
            f"exponent {MAX_POW_EXPONENT + 1} exceeds limit {MAX_POW_EXPONENT}",
            str(cm.exception),
        )

    def test_vm_host_math_pow_rejects_base_above_limit(self) -> None:
        base = MAX_POW_BASE + 1
        bc = compile_to_bytecode(f"print host.math.pow({base}, 2)\n")
        with self.assertRaises(AxiomRuntimeError) as cm:
            Vm(locals_count=bc.locals_count).run(bc, io.StringIO())
        self.assertIn(
            f"base {base} exceeds absolute limit {MAX_POW_BASE}",
            str(cm.exception),
        )

    def test_vm_host_call_by_name(self) -> None:
        bc = compile_to_bytecode("print host.abs(-12)\n")
        host_calls = [item for item in bc.instructions if item.op == Op.HOST_CALL]
        self.assertEqual(len(host_calls), 1)
        host_name_index = host_calls[0].arg
        self.assertIsNotNone(host_name_index)
        self.assertEqual(bc.strings[int(host_name_index)], "abs")

    def test_string_literals_roundtrip_through_bytecode(self) -> None:
        bc = compile_to_bytecode('print "hello\\naxiom"\n')
        string_ops = [item for item in bc.instructions if item.op == Op.CONST_STRING]
        self.assertEqual(len(string_ops), 1)
        self.assertEqual(bc.strings[int(string_ops[0].arg)], "hello\naxiom")

        out = io.StringIO()
        Vm(locals_count=bc.locals_count).run(bc, out)
        self.assertEqual(out.getvalue(), "hello\naxiom\n")

    def test_bool_literals_roundtrip_through_bytecode(self) -> None:
        bc = compile_to_bytecode("print true\n")
        bool_ops = [item for item in bc.instructions if item.op == Op.CONST_BOOL]
        self.assertEqual(len(bool_ops), 1)
        self.assertEqual(int(bool_ops[0].arg), 1)

        out = io.StringIO()
        Vm(locals_count=bc.locals_count).run(bc, out)
        self.assertEqual(out.getvalue(), "true\n")

    def test_bytecode_decode_keeps_v7_compatibility(self) -> None:
        legacy = Bytecode(
            strings=[],
            instructions=[Instr(Op.CONST_I64, 41), Instr(Op.PRINT), Instr(Op.HALT)],
            locals_count=0,
            functions=[],
            version_minor=7,
        )
        decoded = Bytecode.decode(legacy.encode())
        self.assertEqual(decoded.version_minor, 7)

        out = io.StringIO()
        Vm(locals_count=decoded.locals_count).run(decoded, out)
        self.assertEqual(out.getvalue(), "41\n")

    def test_bytecode_decode_keeps_v8_compatibility(self) -> None:
        legacy = Bytecode(
            strings=["hi"],
            instructions=[Instr(Op.CONST_STRING, 0), Instr(Op.PRINT), Instr(Op.HALT)],
            locals_count=0,
            functions=[],
            version_minor=8,
        )
        decoded = Bytecode.decode(legacy.encode())
        self.assertEqual(decoded.version_minor, 8)

        out = io.StringIO()
        Vm(locals_count=decoded.locals_count).run(decoded, out)
        self.assertEqual(out.getvalue(), "hi\n")

    def test_vm_rejects_jump_target_past_instruction_end(self) -> None:
        bc = Bytecode(
            strings=[],
            instructions=[Instr(Op.JMP, 2), Instr(Op.HALT)],
            locals_count=0,
            functions=[],
        )

        with self.assertRaises(AxiomRuntimeError) as cm:
            Vm(locals_count=bc.locals_count).run(bc, io.StringIO())

        self.assertIn(
            "jump target 2 out of bounds (instruction count 2)",
            str(cm.exception),
        )

    def test_vm_rejects_negative_jump_target(self) -> None:
        bc = Bytecode(
            strings=[],
            instructions=[Instr(Op.JMP, -1), Instr(Op.HALT)],
            locals_count=0,
            functions=[],
        )

        with self.assertRaises(AxiomRuntimeError) as cm:
            Vm(locals_count=bc.locals_count).run(bc, io.StringIO())

        self.assertIn(
            "jump target -1 out of bounds (instruction count 2)",
            str(cm.exception),
        )

    def test_vm_rejects_conditional_jump_target_past_instruction_end(self) -> None:
        bc = Bytecode(
            strings=[],
            instructions=[
                Instr(Op.CONST_BOOL, 0),
                Instr(Op.JMP_IF_FALSE, 3),
                Instr(Op.HALT),
            ],
            locals_count=0,
            functions=[],
        )

        with self.assertRaises(AxiomRuntimeError) as cm:
            Vm(locals_count=bc.locals_count).run(bc, io.StringIO())

        self.assertIn(
            "conditional jump target 3 out of bounds (instruction count 3)",
            str(cm.exception),
        )

    def test_vm_rejects_call_entry_past_instruction_end(self) -> None:
        bc = Bytecode(
            strings=["f"],
            instructions=[Instr(Op.CALL, 0), Instr(Op.HALT)],
            locals_count=0,
            functions=[FunctionMeta(name_index=0, entry=2, arity=0, locals_count=0)],
        )

        with self.assertRaises(AxiomRuntimeError) as cm:
            Vm(locals_count=bc.locals_count).run(bc, io.StringIO())

        self.assertIn(
            "call target 0 entry 2 out of bounds (instruction count 2)",
            str(cm.exception),
        )

    def test_vm_rejects_indirect_call_entry_past_instruction_end(self) -> None:
        bc = Bytecode(
            strings=["f"],
            instructions=[
                Instr(Op.LOAD_FN, 0),
                Instr(Op.CALL_INDIRECT, 0),
                Instr(Op.HALT),
            ],
            locals_count=0,
            functions=[FunctionMeta(name_index=0, entry=3, arity=0, locals_count=0)],
        )

        with self.assertRaises(AxiomRuntimeError) as cm:
            Vm(locals_count=bc.locals_count).run(bc, io.StringIO())

        self.assertIn(
            "indirect call target 0 entry 3 out of bounds (instruction count 3)",
            str(cm.exception),
        )

    def test_runtime_host_print_requires_explicit_allow(self) -> None:
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(parse_program("host.print(1)\n"), io.StringIO())

    def test_runtime_host_print_requires_explicit_allow_vm(self) -> None:
        bc = compile_to_bytecode("host.print(1)\n", allow_host_side_effects=True)
        with self.assertRaises(AxiomRuntimeError):
            Vm(locals_count=bc.locals_count).run(bc, io.StringIO())

    def test_runtime_host_print_with_allow(self) -> None:
        out = io.StringIO()
        Interpreter(allow_host_side_effects=True).run(parse_program("host.print(1)\n"), out)
        self.assertEqual(out.getvalue(), "1\n")

    def test_runtime_host_print_with_allow_vm(self) -> None:
        bc = compile_to_bytecode("host.print(1)\n", allow_host_side_effects=True)
        out = io.StringIO()
        Vm(locals_count=bc.locals_count, allow_host_side_effects=True).run(bc, out)
        self.assertEqual(out.getvalue(), "1\n")

    def test_runtime_host_print_string_with_allow_vm(self) -> None:
        bc = compile_to_bytecode('host.print("hi")\n', allow_host_side_effects=True)
        out = io.StringIO()
        Vm(locals_count=bc.locals_count, allow_host_side_effects=True).run(bc, out)
        self.assertEqual(out.getvalue(), "hi\n")

    def test_runtime_non_host_namespace_call(self) -> None:
        with self.assertRaises(AxiomParseError):
            parse_program("foo.bar(1)\n")

    def test_runtime_reserved_host_identifier_let(self) -> None:
        program = Program(
            [
                LetStmt(
                    name="host",
                    type_ref=TypeRef(name="int", span=Span(0, 1)),
                    expr=IntLit(1, Span(0, 1)),
                    span=Span(0, 1),
                )
            ]
        )
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(program, io.StringIO())

    def test_runtime_reserved_host_identifier_param(self) -> None:
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
                        stmts=[
                            ReturnStmt(expr=VarRef(name="host", span=Span(0, 4)), span=Span(0, 4))
                        ],
                        span=Span(0, 6),
                    ),
                    span=Span(0, 8),
                )
            ]
        )
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(program, io.StringIO())

    def test_runtime_reserved_host_identifier_assign(self) -> None:
        program = Program(
            [
                LetStmt(
                    name="x",
                    type_ref=TypeRef(name="int", span=Span(0, 1)),
                    expr=IntLit(1, Span(0, 1)),
                    span=Span(0, 1),
                ),
                AssignStmt(name="host", expr=IntLit(2, Span(2, 3)), span=Span(2, 4)),
            ]
        )
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(program, io.StringIO())

    def test_runtime_reserved_host_function_name(self) -> None:
        program = Program(
            [
                FunctionDefStmt(
                    name="host",
                    params=[],
                    return_type=TypeRef(name="int", span=Span(0, 1)),
                    body=BlockStmt(
                        stmts=[ReturnStmt(expr=IntLit(0, Span(0, 1)), span=Span(0, 1))],
                        span=Span(0, 3),
                    ),
                    span=Span(0, 5),
                )
            ]
        )
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(program, io.StringIO())

    @patch("builtins.input", return_value="41")
    def test_runtime_host_read_with_allow(self, fake_input) -> None:
        out = io.StringIO()
        Interpreter(allow_host_side_effects=True).run(parse_program("print host.read(123)\n"), out)
        self.assertEqual(out.getvalue(), "41\n")
        fake_input.assert_called_once_with("123")

    @patch("builtins.input", return_value="41")
    def test_runtime_host_read_with_allow_vm(self, fake_input) -> None:
        bc = compile_to_bytecode("print host.read(123)\n", allow_host_side_effects=True)
        out = io.StringIO()
        Vm(locals_count=bc.locals_count, allow_host_side_effects=True).run(bc, out)
        self.assertEqual(out.getvalue(), "41\n")
        fake_input.assert_called_once_with("123")

    @patch("builtins.input", return_value="41")
    def test_runtime_host_read_parse_int_with_allow_vm(self, fake_input) -> None:
        bc = compile_to_bytecode(
            'print host.int.parse(host.read("num> "))\n',
            allow_host_side_effects=True,
        )
        out = io.StringIO()
        Vm(locals_count=bc.locals_count, allow_host_side_effects=True).run(bc, out)
        self.assertEqual(out.getvalue(), "41\n")
        fake_input.assert_called_once_with("num> ")

    def test_runtime_string_condition_requires_bool(self) -> None:
        src = """
fn choose(cond: string): int {
  if cond {
    print 1
  }
  return 0
}

choose("yes")
"""
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(parse_program(src), io.StringIO())

        with self.assertRaises(AxiomCompileError) as cm:
            compile_to_bytecode(src)
        self.assertIn("if condition expects bool, got string", str(cm.exception))

    def test_host_contract_signature_tracks_capability_state(self) -> None:
        base_signature = host_contract_metadata()["capabilities_signature"]

        def probe(_args: list[int], _out) -> int:
            return 7

        register_host_builtin("sig_probe", 0, False, probe)
        try:
            with_probe = host_contract_metadata()
            self.assertNotEqual(base_signature, with_probe["capabilities_signature"])
            self.assertIn("capabilities_signature", with_probe)
            self.assertEqual(with_probe["schema_version"], 1)
            self.assertIn("sig_probe", {entry["name"] for entry in with_probe["capabilities"]})
        finally:
            reset_host_builtins()


if __name__ == "__main__":
    unittest.main()
