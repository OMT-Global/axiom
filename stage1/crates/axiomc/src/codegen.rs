use crate::diagnostics::Diagnostic;
use crate::mir::{
    EnumDef, Expr, Function, LiteralValue, MatchArm, Param, Program, Stmt, StructDef, StructField,
    Type,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

pub fn render_rust(program: &Program) -> String {
    let type_context = TypeContext::new(program);
    let mut out = String::new();
    out.push_str("#[allow(unused_imports)]\n");
    out.push_str("use std::collections::HashMap;\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_array_get<T: Copy>(values: &[T], index: i64) -> T {\n");
    out.push_str(
        "    let index = usize::try_from(index).expect(\"array index must be non-negative\");\n",
    );
    out.push_str("    values[index]\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_array_take<T>(values: Vec<T>, index: i64) -> T {\n");
    out.push_str(
        "    let index = usize::try_from(index).expect(\"array index must be non-negative\");\n",
    );
    out.push_str("    values.into_iter().nth(index).expect(\"array index out of bounds\")\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str(
        "fn axiom_array_slice_bounds(len: usize, start: Option<i64>, end: Option<i64>) -> (usize, usize) {\n",
    );
    out.push_str("    let start = start.unwrap_or(0);\n");
    out.push_str("    let end = end.unwrap_or(len as i64);\n");
    out.push_str(
        "    let start = usize::try_from(start).expect(\"array slice start must be non-negative\");\n",
    );
    out.push_str(
        "    let end = usize::try_from(end).expect(\"array slice end must be non-negative\");\n",
    );
    out.push_str("    assert!(start <= end, \"array slice start must be <= end\");\n");
    out.push_str("    assert!(end <= len, \"array slice end out of bounds\");\n");
    out.push_str("    (start, end)\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_slice_view<'a, T>(values: &'a [T], start: Option<i64>, end: Option<i64>) -> &'a [T] {\n");
    out.push_str("    let (start, end) = axiom_array_slice_bounds(values.len(), start, end);\n");
    out.push_str("    &values[start..end]\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_slice_view_mut<'a, T>(values: &'a mut [T], start: Option<i64>, end: Option<i64>) -> &'a mut [T] {\n");
    out.push_str("    let (start, end) = axiom_array_slice_bounds(values.len(), start, end);\n");
    out.push_str("    &mut values[start..end]\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_last_index(len: usize) -> i64 {\n");
    out.push_str("    assert!(len > 0, \"collection must not be empty\");\n");
    out.push_str("    (len - 1) as i64\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_io_eprintln(text: String) -> i64 {\n");
    out.push_str("    use std::io::Write;\n");
    out.push_str("    let stderr = std::io::stderr();\n");
    out.push_str("    let mut handle = stderr.lock();\n");
    out.push_str(
        "    match handle.write_all(text.as_bytes()).and_then(|_| handle.write_all(b\"\\n\")) {\n",
    );
    out.push_str("        Ok(()) => (text.len() as i64) + 1,\n");
    out.push_str("        Err(_) => -1,\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_fs_read(path: String) -> Option<String> {\n");
    out.push_str("    std::fs::read_to_string(path).ok()\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_net_resolve(host: String) -> Option<String> {\n");
    out.push_str("    use std::net::ToSocketAddrs;\n");
    out.push_str("    (host.as_str(), 0)\n");
    out.push_str("        .to_socket_addrs()\n");
    out.push_str("        .ok()?\n");
    out.push_str("        .next()\n");
    out.push_str("        .map(|addr| addr.ip().to_string())\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_http_get(url: String) -> Option<String> {\n");
    out.push_str("    use std::io::{Read, Write};\n");
    out.push_str("    use std::net::TcpStream;\n");
    out.push_str("    use std::time::Duration;\n");
    out.push_str("    // Stage1 HTTP client: http:// only, HTTP/1.0 with\n");
    out.push_str("    // Connection: close so we can read the body to EOF\n");
    out.push_str("    // without parsing Content-Length or chunked transfer\n");
    out.push_str("    // encoding. Returns the response body on 2xx, None on\n");
    out.push_str("    // any parse / connect / non-2xx error. HTTPS and TLS\n");
    out.push_str("    // are deliberately out of scope at this slice.\n");
    out.push_str("    let rest = url.strip_prefix(\"http://\")?;\n");
    out.push_str("    let (host_port, path) = match rest.find('/') {\n");
    out.push_str("        Some(idx) => (&rest[..idx], &rest[idx..]),\n");
    out.push_str("        None => (rest, \"/\"),\n");
    out.push_str("    };\n");
    out.push_str("    if host_port.is_empty() {\n");
    out.push_str("        return None;\n");
    out.push_str("    }\n");
    out.push_str("    let (host, port) = match host_port.rfind(':') {\n");
    out.push_str("        Some(idx) => {\n");
    out.push_str("            let parsed: u16 = host_port[idx + 1..].parse().ok()?;\n");
    out.push_str("            (&host_port[..idx], parsed)\n");
    out.push_str("        }\n");
    out.push_str("        None => (host_port, 80u16),\n");
    out.push_str("    };\n");
    out.push_str("    let mut stream = TcpStream::connect((host, port)).ok()?;\n");
    out.push_str("    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;\n");
    out.push_str("    stream.set_write_timeout(Some(Duration::from_secs(5))).ok()?;\n");
    out.push_str("    let request = format!(\n");
    out.push_str("        \"GET {} HTTP/1.0\\r\\nHost: {}\\r\\nUser-Agent: axiom-stage1/0.1\\r\\nConnection: close\\r\\n\\r\\n\",\n");
    out.push_str("        path, host\n");
    out.push_str("    );\n");
    out.push_str("    stream.write_all(request.as_bytes()).ok()?;\n");
    out.push_str("    let mut raw = Vec::new();\n");
    out.push_str("    stream.read_to_end(&mut raw).ok()?;\n");
    out.push_str("    let sep = raw.windows(4).position(|w| w == b\"\\r\\n\\r\\n\")?;\n");
    out.push_str("    let head = &raw[..sep];\n");
    out.push_str("    let body = &raw[sep + 4..];\n");
    out.push_str(
        "    let status_line_end = head.iter().position(|b| *b == b'\\r').unwrap_or(head.len());\n",
    );
    out.push_str("    let status_line = std::str::from_utf8(&head[..status_line_end]).ok()?;\n");
    out.push_str("    let mut parts = status_line.splitn(3, ' ');\n");
    out.push_str("    let _version = parts.next()?;\n");
    out.push_str("    let status_code: u16 = parts.next()?.parse().ok()?;\n");
    out.push_str("    if !(200..300).contains(&status_code) {\n");
    out.push_str("        return None;\n");
    out.push_str("    }\n");
    out.push_str("    String::from_utf8(body.to_vec()).ok()\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_process_status(program: String) -> i64 {\n");
    out.push_str("    std::process::Command::new(program)\n");
    out.push_str("        .status()\n");
    out.push_str("        .ok()\n");
    out.push_str("        .and_then(|status| status.code())\n");
    out.push_str("        .map(i64::from)\n");
    out.push_str("        .unwrap_or(-1)\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_clock_now_ms() -> i64 {\n");
    out.push_str("    use std::time::{SystemTime, UNIX_EPOCH};\n");
    out.push_str("    let now = SystemTime::now()\n");
    out.push_str("        .duration_since(UNIX_EPOCH)\n");
    out.push_str("        .expect(\"system clock must be after unix epoch\");\n");
    out.push_str("    now.as_millis() as i64\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_env_get(name: String) -> Option<String> {\n");
    out.push_str("    std::env::var(name).ok()\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_crypto_sha256(input: String) -> String {\n");
    out.push_str("    const K: [u32; 64] = [\n");
    out.push_str("        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,\n");
    out.push_str("        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,\n");
    out.push_str("        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,\n");
    out.push_str("        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,\n");
    out.push_str("        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,\n");
    out.push_str("        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,\n");
    out.push_str("        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,\n");
    out.push_str("        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,\n");
    out.push_str("        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,\n");
    out.push_str("        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,\n");
    out.push_str("        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,\n");
    out.push_str("        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,\n");
    out.push_str("        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,\n");
    out.push_str("        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,\n");
    out.push_str("        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,\n");
    out.push_str("        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,\n");
    out.push_str("    ];\n");
    out.push_str("    let mut state: [u32; 8] = [\n");
    out.push_str("        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,\n");
    out.push_str("        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,\n");
    out.push_str("    ];\n");
    out.push_str("    let mut data = input.into_bytes();\n");
    out.push_str("    let bit_len = (data.len() as u64) * 8;\n");
    out.push_str("    data.push(0x80);\n");
    out.push_str("    while data.len() % 64 != 56 {\n");
    out.push_str("        data.push(0);\n");
    out.push_str("    }\n");
    out.push_str("    data.extend_from_slice(&bit_len.to_be_bytes());\n");
    out.push_str("    for chunk in data.chunks(64) {\n");
    out.push_str("        let mut schedule = [0u32; 64];\n");
    out.push_str("        for (index, word) in schedule.iter_mut().take(16).enumerate() {\n");
    out.push_str("            let start = index * 4;\n");
    out.push_str("            *word = u32::from_be_bytes([\n");
    out.push_str("                chunk[start],\n");
    out.push_str("                chunk[start + 1],\n");
    out.push_str("                chunk[start + 2],\n");
    out.push_str("                chunk[start + 3],\n");
    out.push_str("            ]);\n");
    out.push_str("        }\n");
    out.push_str("        for index in 16..64 {\n");
    out.push_str("            let s0 = schedule[index - 15].rotate_right(7)\n");
    out.push_str("                ^ schedule[index - 15].rotate_right(18)\n");
    out.push_str("                ^ (schedule[index - 15] >> 3);\n");
    out.push_str("            let s1 = schedule[index - 2].rotate_right(17)\n");
    out.push_str("                ^ schedule[index - 2].rotate_right(19)\n");
    out.push_str("                ^ (schedule[index - 2] >> 10);\n");
    out.push_str("            schedule[index] = schedule[index - 16]\n");
    out.push_str("                .wrapping_add(s0)\n");
    out.push_str("                .wrapping_add(schedule[index - 7])\n");
    out.push_str("                .wrapping_add(s1);\n");
    out.push_str("        }\n");
    out.push_str("        let mut a = state[0];\n");
    out.push_str("        let mut b = state[1];\n");
    out.push_str("        let mut c = state[2];\n");
    out.push_str("        let mut d = state[3];\n");
    out.push_str("        let mut e = state[4];\n");
    out.push_str("        let mut f = state[5];\n");
    out.push_str("        let mut g = state[6];\n");
    out.push_str("        let mut h = state[7];\n");
    out.push_str("        for index in 0..64 {\n");
    out.push_str(
        "            let sigma1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);\n",
    );
    out.push_str("            let choice = (e & f) ^ ((!e) & g);\n");
    out.push_str("            let temp1 = h\n");
    out.push_str("                .wrapping_add(sigma1)\n");
    out.push_str("                .wrapping_add(choice)\n");
    out.push_str("                .wrapping_add(K[index])\n");
    out.push_str("                .wrapping_add(schedule[index]);\n");
    out.push_str(
        "            let sigma0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);\n",
    );
    out.push_str("            let majority = (a & b) ^ (a & c) ^ (b & c);\n");
    out.push_str("            let temp2 = sigma0.wrapping_add(majority);\n");
    out.push_str("            h = g;\n");
    out.push_str("            g = f;\n");
    out.push_str("            f = e;\n");
    out.push_str("            e = d.wrapping_add(temp1);\n");
    out.push_str("            d = c;\n");
    out.push_str("            c = b;\n");
    out.push_str("            b = a;\n");
    out.push_str("            a = temp1.wrapping_add(temp2);\n");
    out.push_str("        }\n");
    out.push_str("        state[0] = state[0].wrapping_add(a);\n");
    out.push_str("        state[1] = state[1].wrapping_add(b);\n");
    out.push_str("        state[2] = state[2].wrapping_add(c);\n");
    out.push_str("        state[3] = state[3].wrapping_add(d);\n");
    out.push_str("        state[4] = state[4].wrapping_add(e);\n");
    out.push_str("        state[5] = state[5].wrapping_add(f);\n");
    out.push_str("        state[6] = state[6].wrapping_add(g);\n");
    out.push_str("        state[7] = state[7].wrapping_add(h);\n");
    out.push_str("    }\n");
    out.push_str("    let mut output = String::new();\n");
    out.push_str("    for value in state {\n");
    out.push_str("        output.push_str(&format!(\"{value:08x}\"));\n");
    out.push_str("    }\n");
    out.push_str("    output\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str(
        "fn axiom_map_get<K: Eq + std::hash::Hash, V: Copy>(values: &HashMap<K, V>, key: &K) -> V {\n",
    );
    out.push_str("    *values.get(key).expect(\"map key not found\")\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str(
        "fn axiom_map_take<K: Eq + std::hash::Hash, V>(mut values: HashMap<K, V>, key: &K) -> V {\n",
    );
    out.push_str("    values.remove(key).expect(\"map key not found\")\n");
    out.push_str("}\n\n");
    for enum_def in &program.enums {
        render_enum(enum_def, &type_context, &mut out);
        out.push('\n');
    }
    for struct_def in &program.structs {
        render_struct(struct_def, &type_context, &mut out);
        out.push('\n');
    }
    for function in &program.functions {
        render_function(function, &type_context, &mut out);
        out.push('\n');
    }
    out.push_str("fn main() {\n");
    for stmt in &program.stmts {
        render_stmt(stmt, &type_context, &mut out, 1);
    }
    out.push_str("}\n");
    out
}

struct TypeContext<'a> {
    structs: HashMap<&'a str, &'a StructDef>,
    enums: HashMap<&'a str, &'a EnumDef>,
}

impl<'a> TypeContext<'a> {
    fn new(program: &'a Program) -> Self {
        Self {
            structs: program
                .structs
                .iter()
                .map(|struct_def| (struct_def.name.as_str(), struct_def))
                .collect(),
            enums: program
                .enums
                .iter()
                .map(|enum_def| (enum_def.name.as_str(), enum_def))
                .collect(),
        }
    }

    fn type_contains_borrowed_slice(&self, ty: &Type) -> bool {
        self.type_contains_borrowed_slice_inner(ty, &mut HashSet::new(), &mut HashSet::new())
    }

    fn struct_uses_borrowed_slice(&self, name: &str) -> bool {
        self.type_contains_borrowed_slice(&Type::Struct(name.to_string()))
    }

    fn enum_uses_borrowed_slice(&self, name: &str) -> bool {
        self.type_contains_borrowed_slice(&Type::Enum(name.to_string()))
    }

    fn type_contains_borrowed_slice_inner(
        &self,
        ty: &Type,
        visiting_structs: &mut HashSet<String>,
        visiting_enums: &mut HashSet<String>,
    ) -> bool {
        match ty {
            Type::Int | Type::Bool | Type::String => false,
            Type::Slice(_) | Type::MutSlice(_) => true,
            Type::Struct(name) => {
                if !visiting_structs.insert(name.clone()) {
                    return false;
                }
                let contains = self.structs.get(name.as_str()).is_some_and(|struct_def| {
                    struct_def.fields.iter().any(|field| {
                        self.type_contains_borrowed_slice_inner(
                            &field.ty,
                            visiting_structs,
                            visiting_enums,
                        )
                    })
                });
                visiting_structs.remove(name);
                contains
            }
            Type::Enum(name) => {
                if !visiting_enums.insert(name.clone()) {
                    return false;
                }
                let contains = self.enums.get(name.as_str()).is_some_and(|enum_def| {
                    enum_def.variants.iter().any(|variant| {
                        variant.payload_tys.iter().any(|payload_ty| {
                            self.type_contains_borrowed_slice_inner(
                                payload_ty,
                                visiting_structs,
                                visiting_enums,
                            )
                        })
                    })
                });
                visiting_enums.remove(name);
                contains
            }
            Type::Option(inner) => {
                self.type_contains_borrowed_slice_inner(inner, visiting_structs, visiting_enums)
            }
            Type::Result(ok, err) => {
                self.type_contains_borrowed_slice_inner(ok, visiting_structs, visiting_enums)
                    || self.type_contains_borrowed_slice_inner(
                        err,
                        visiting_structs,
                        visiting_enums,
                    )
            }
            Type::Tuple(elements) => elements.iter().any(|element| {
                self.type_contains_borrowed_slice_inner(element, visiting_structs, visiting_enums)
            }),
            Type::Map(key, value) => {
                self.type_contains_borrowed_slice_inner(key, visiting_structs, visiting_enums)
                    || self.type_contains_borrowed_slice_inner(
                        value,
                        visiting_structs,
                        visiting_enums,
                    )
            }
            Type::Array(inner) => {
                self.type_contains_borrowed_slice_inner(inner, visiting_structs, visiting_enums)
            }
        }
    }
}

fn render_struct(struct_def: &StructDef, type_context: &TypeContext<'_>, out: &mut String) {
    let lifetime = if type_context.struct_uses_borrowed_slice(&struct_def.name) {
        "<'a>"
    } else {
        ""
    };
    out.push_str("#[allow(non_camel_case_types)]\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("#[derive(Debug, PartialEq)]\n");
    out.push_str(&format!("struct {}{} {{\n", struct_def.name, lifetime));
    for field in &struct_def.fields {
        render_struct_field(field, type_context, out, 1, !lifetime.is_empty());
    }
    out.push_str("}\n");
}

fn render_enum(enum_def: &EnumDef, type_context: &TypeContext<'_>, out: &mut String) {
    let lifetime = if type_context.enum_uses_borrowed_slice(&enum_def.name) {
        "<'a>"
    } else {
        ""
    };
    out.push_str("#[allow(non_camel_case_types)]\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("#[derive(Debug, PartialEq)]\n");
    out.push_str(&format!("enum {}{} {{\n", enum_def.name, lifetime));
    for variant in &enum_def.variants {
        if variant.payload_tys.is_empty() {
            out.push_str(&format!("    {},\n", variant.name));
        } else if !variant.payload_names.is_empty() {
            out.push_str(&format!("    {} {{\n", variant.name));
            for (payload_name, payload_ty) in
                variant.payload_names.iter().zip(variant.payload_tys.iter())
            {
                out.push_str(&format!(
                    "        {}: {},\n",
                    payload_name,
                    rust_type_inner(payload_ty, Some("'a"), type_context)
                ));
            }
            out.push_str("    },\n");
        } else {
            let payload_tys = variant
                .payload_tys
                .iter()
                .map(|payload_ty| rust_type_inner(payload_ty, Some("'a"), type_context))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("    {}({payload_tys}),\n", variant.name));
        }
    }
    out.push_str("}\n");
}

fn render_struct_field(
    field: &StructField,
    type_context: &TypeContext<'_>,
    out: &mut String,
    indent: usize,
    uses_slice_lifetime: bool,
) {
    let pad = "    ".repeat(indent);
    let lifetime = uses_slice_lifetime.then_some("'a");
    out.push_str(&format!(
        "{pad}{}: {},\n",
        field.name,
        rust_type_inner(&field.ty, lifetime, type_context)
    ));
}

fn render_function(function: &Function, type_context: &TypeContext<'_>, out: &mut String) {
    let uses_slice_lifetime = function_signature_uses_borrowed_slice(function, type_context);
    let params = function
        .params
        .iter()
        .map(|param| render_param(param, uses_slice_lifetime, type_context))
        .collect::<Vec<_>>()
        .join(", ");
    let lifetime = if uses_slice_lifetime { "<'a>" } else { "" };
    out.push_str(&format!(
        "fn {}{}({}) -> {} {{\n",
        function.name,
        lifetime,
        params,
        rust_type_in_signature(&function.return_ty, uses_slice_lifetime, type_context)
    ));
    for stmt in &function.body {
        render_stmt(stmt, type_context, out, 1);
    }
    out.push_str("}\n");
}

fn render_param(
    param: &Param,
    uses_slice_lifetime: bool,
    type_context: &TypeContext<'_>,
) -> String {
    format!(
        "{}: {}",
        param.name,
        rust_type_in_signature(&param.ty, uses_slice_lifetime, type_context)
    )
}

fn render_stmt(stmt: &Stmt, type_context: &TypeContext<'_>, out: &mut String, indent: usize) {
    let pad = "    ".repeat(indent);
    match stmt {
        Stmt::Let { name, ty, expr } => out.push_str(&format!(
            "{pad}let {name}: {} = {};\n",
            rust_type(ty, type_context),
            render_expr(expr)
        )),
        Stmt::Print(expr) => out.push_str(&format!(
            "{pad}println!(\"{{}}\", {});\n",
            render_expr(expr)
        )),
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            out.push_str(&format!("{pad}if {} {{\n", render_expr(cond)));
            for stmt in then_block {
                render_stmt(stmt, type_context, out, indent + 1);
            }
            if let Some(else_block) = else_block {
                out.push_str(&format!("{pad}}} else {{\n"));
                for stmt in else_block {
                    render_stmt(stmt, type_context, out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            } else {
                out.push_str(&format!("{pad}}}\n"));
            }
        }
        Stmt::While { cond, body } => {
            out.push_str(&format!("{pad}while {} {{\n", render_expr(cond)));
            for stmt in body {
                render_stmt(stmt, type_context, out, indent + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        Stmt::Match { expr, arms } => {
            out.push_str(&format!("{pad}match {} {{\n", render_expr(expr)));
            for arm in arms {
                render_match_arm(arm, type_context, out, indent + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        Stmt::Return(expr) => out.push_str(&format!("{pad}return {};\n", render_expr(expr))),
    }
}

fn render_match_arm(
    arm: &MatchArm,
    type_context: &TypeContext<'_>,
    out: &mut String,
    indent: usize,
) {
    let pad = "    ".repeat(indent);
    if arm.bindings.is_empty() {
        out.push_str(&format!("{pad}{}::{} => {{\n", arm.enum_name, arm.variant));
    } else if arm.is_named {
        out.push_str(&format!(
            "{pad}{}::{} {{ {} }} => {{\n",
            arm.enum_name,
            arm.variant,
            arm.bindings.join(", ")
        ));
    } else {
        out.push_str(&format!(
            "{pad}{}::{}({}) => {{\n",
            arm.enum_name,
            arm.variant,
            arm.bindings.join(", ")
        ));
    }
    for stmt in &arm.body {
        render_stmt(stmt, type_context, out, indent + 1);
    }
    out.push_str(&format!("{pad}}},\n"));
}

fn render_expr(expr: &Expr) -> String {
    match expr {
        Expr::Literal(LiteralValue::Int(value)) => value.to_string(),
        Expr::Literal(LiteralValue::Bool(value)) => value.to_string(),
        Expr::Literal(LiteralValue::String(value)) => format!("String::from({value:?})"),
        Expr::VarRef { name, .. } => name.clone(),
        Expr::Call { name, args, .. } if name == "len" => {
            format!("({}).len() as i64", render_expr(&args[0]))
        }
        Expr::Call { name, args, .. } if name == "io_eprintln" => {
            format!("axiom_io_eprintln({})", render_expr(&args[0]))
        }
        Expr::Call { name, args, .. } if name == "fs_read" => {
            format!("axiom_fs_read({})", render_expr(&args[0]))
        }
        Expr::Call { name, args, .. } if name == "http_get" => {
            format!("axiom_http_get({})", render_expr(&args[0]))
        }
        Expr::Call { name, args, .. } if name == "net_resolve" => {
            format!("axiom_net_resolve({})", render_expr(&args[0]))
        }
        Expr::Call { name, args, .. } if name == "process_status" => {
            format!("axiom_process_status({})", render_expr(&args[0]))
        }
        Expr::Call { name, .. } if name == "clock_now_ms" => String::from("axiom_clock_now_ms()"),
        Expr::Call { name, args, .. } if name == "env_get" => {
            format!("axiom_env_get({})", render_expr(&args[0]))
        }
        Expr::Call { name, args, .. } if name == "crypto_sha256" => {
            format!("axiom_crypto_sha256({})", render_expr(&args[0]))
        }
        Expr::Call { name, args, ty } if name == "first" => {
            render_collection_edge(&args[0], ty, false)
        }
        Expr::Call { name, args, ty } if name == "last" => {
            render_collection_edge(&args[0], ty, true)
        }
        Expr::Call { name, args, .. } => {
            let rendered_args = args.iter().map(render_expr).collect::<Vec<_>>().join(", ");
            format!("{name}({rendered_args})")
        }
        Expr::BinaryAdd { lhs, rhs, ty } => match ty {
            Type::Int => format!("{} + {}", render_expr(lhs), render_expr(rhs)),
            Type::String => format!(
                "format!(\"{{}}{{}}\", {}, {})",
                render_expr(lhs),
                render_expr(rhs)
            ),
            Type::Bool => unreachable!("type checker rejects bool addition"),
            Type::Struct(_) => unreachable!("type checker rejects struct addition"),
            Type::Enum(_) => unreachable!("type checker rejects enum addition"),
            Type::Slice(_) | Type::MutSlice(_) => {
                unreachable!("type checker rejects slice addition")
            }
            Type::Option(_) => unreachable!("type checker rejects option addition"),
            Type::Result(_, _) => unreachable!("type checker rejects result addition"),
            Type::Tuple(_) => unreachable!("type checker rejects tuple addition"),
            Type::Map(_, _) => unreachable!("type checker rejects map addition"),
            Type::Array(_) => unreachable!("type checker rejects array addition"),
        },
        Expr::BinaryCompare { op, lhs, rhs, .. } => {
            format!("{} {} {}", render_expr(lhs), op.lexeme(), render_expr(rhs))
        }
        Expr::Stringify { expr, .. } => format!("({}).to_string()", render_expr(expr)),
        Expr::StructLiteral { name, fields, .. } => {
            let rendered_fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, render_expr(&field.expr)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name} {{ {rendered_fields} }}")
        }
        Expr::FieldAccess { base, field, .. } => format!("({}).{}", render_expr(base), field),
        Expr::TupleLiteral { elements, .. } => {
            let rendered = elements
                .iter()
                .map(render_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({rendered})")
        }
        Expr::TupleIndex { base, index, .. } => format!("({}).{}", render_expr(base), index),
        Expr::MapLiteral { entries, .. } => {
            let rendered = entries
                .iter()
                .map(|entry| {
                    format!(
                        "({}, {})",
                        render_expr(&entry.key),
                        render_expr(&entry.value)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("HashMap::from([{rendered}])")
        }
        Expr::EnumVariant {
            enum_name,
            variant,
            field_names,
            payloads,
            ..
        } => {
            if payloads.is_empty() {
                format!("{enum_name}::{variant}")
            } else if !field_names.is_empty() {
                let rendered_fields = field_names
                    .iter()
                    .zip(payloads.iter())
                    .map(|(field_name, payload)| format!("{field_name}: {}", render_expr(payload)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{enum_name}::{variant} {{ {rendered_fields} }}")
            } else {
                let rendered_payloads = payloads
                    .iter()
                    .map(render_expr)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{enum_name}::{variant}({rendered_payloads})")
            }
        }
        Expr::ArrayLiteral { elements, .. } => {
            let rendered = elements
                .iter()
                .map(render_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("vec![{rendered}]")
        }
        Expr::Slice {
            base, start, end, ..
        } => {
            let start = start
                .as_ref()
                .map(|expr| format!("Some({})", render_expr(expr)))
                .unwrap_or_else(|| String::from("None"));
            let end = end
                .as_ref()
                .map(|expr| format!("Some({})", render_expr(expr)))
                .unwrap_or_else(|| String::from("None"));
            match base.ty() {
                Type::Array(_) => {
                    if matches!(expr.ty(), Type::MutSlice(_)) {
                        format!(
                            "axiom_slice_view_mut(&mut {}, {}, {})",
                            render_expr(base),
                            start,
                            end
                        )
                    } else {
                        format!(
                            "axiom_slice_view(&{}, {}, {})",
                            render_expr(base),
                            start,
                            end
                        )
                    }
                }
                Type::Slice(_) | Type::MutSlice(_) => {
                    if matches!(expr.ty(), Type::MutSlice(_)) {
                        format!(
                            "axiom_slice_view_mut({}, {}, {})",
                            render_expr(base),
                            start,
                            end
                        )
                    } else {
                        format!(
                            "axiom_slice_view({}, {}, {})",
                            render_expr(base),
                            start,
                            end
                        )
                    }
                }
                _ => unreachable!("type checker rejects slicing non-array values"),
            }
        }
        Expr::Index { base, index, ty } => match base.ty() {
            Type::Array(_) => {
                if ty.is_copy() {
                    format!(
                        "axiom_array_get(&{}, {})",
                        render_expr(base),
                        render_expr(index)
                    )
                } else {
                    format!(
                        "axiom_array_take({}, {})",
                        render_expr(base),
                        render_expr(index)
                    )
                }
            }
            Type::Slice(_) | Type::MutSlice(_) => {
                format!(
                    "axiom_array_get({}, {})",
                    render_expr(base),
                    render_expr(index)
                )
            }
            Type::Map(_, _) => {
                if ty.is_copy() {
                    format!(
                        "axiom_map_get(&{}, &{})",
                        render_expr(base),
                        render_expr(index)
                    )
                } else {
                    format!(
                        "axiom_map_take({}, &{})",
                        render_expr(base),
                        render_expr(index)
                    )
                }
            }
            _ => unreachable!("type checker rejects indexing non-collection values"),
        },
    }
}

fn rust_type(ty: &Type, type_context: &TypeContext<'_>) -> String {
    rust_type_inner(ty, None, type_context)
}

fn rust_type_in_signature(
    ty: &Type,
    uses_slice_lifetime: bool,
    type_context: &TypeContext<'_>,
) -> String {
    if uses_slice_lifetime {
        rust_type_inner(ty, Some("'a"), type_context)
    } else {
        rust_type(ty, type_context)
    }
}

fn rust_type_inner(ty: &Type, lifetime: Option<&str>, type_context: &TypeContext<'_>) -> String {
    match ty {
        Type::Int => String::from("i64"),
        Type::Bool => String::from("bool"),
        Type::String => String::from("String"),
        Type::Struct(name) => {
            if type_context.struct_uses_borrowed_slice(name) {
                format!("{name}<{}>", lifetime.unwrap_or("'_"))
            } else {
                name.clone()
            }
        }
        Type::Enum(name) => {
            if type_context.enum_uses_borrowed_slice(name) {
                format!("{name}<{}>", lifetime.unwrap_or("'_"))
            } else {
                name.clone()
            }
        }
        Type::Slice(inner) => {
            let inner = rust_type_inner(inner, lifetime, type_context);
            match lifetime {
                Some(lifetime) => format!("&{lifetime} [{inner}]"),
                None => format!("&[{inner}]"),
            }
        }
        Type::MutSlice(inner) => {
            let inner = rust_type_inner(inner, lifetime, type_context);
            match lifetime {
                Some(lifetime) => format!("&{lifetime} mut [{inner}]"),
                None => format!("&mut [{inner}]"),
            }
        }
        Type::Option(inner) => {
            format!("Option<{}>", rust_type_inner(inner, lifetime, type_context))
        }
        Type::Result(ok, err) => format!(
            "Result<{}, {}>",
            rust_type_inner(ok, lifetime, type_context),
            rust_type_inner(err, lifetime, type_context)
        ),
        Type::Tuple(elements) => format!(
            "({})",
            elements
                .iter()
                .map(|element| rust_type_inner(element, lifetime, type_context))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::Map(key, value) => format!(
            "HashMap<{}, {}>",
            rust_type_inner(key, lifetime, type_context),
            rust_type_inner(value, lifetime, type_context)
        ),
        Type::Array(inner) => format!("Vec<{}>", rust_type_inner(inner, lifetime, type_context)),
    }
}

fn function_signature_uses_borrowed_slice(
    function: &Function,
    type_context: &TypeContext<'_>,
) -> bool {
    type_context.type_contains_borrowed_slice(&function.return_ty)
        || function
            .params
            .iter()
            .any(|param| type_context.type_contains_borrowed_slice(&param.ty))
}

fn render_collection_edge(collection: &Expr, result_ty: &Type, from_end: bool) -> String {
    let rendered = render_expr(collection);
    let index = if from_end {
        String::from("axiom_last_index(values.len())")
    } else {
        String::from("0")
    };
    match collection.ty() {
        Type::Array(_) => {
            if result_ty.is_copy() {
                format!("{{ let values = {rendered}; axiom_array_get(&values, {index}) }}")
            } else {
                format!(
                    "{{ let values = {rendered}; let index = {index}; axiom_array_take(values, index) }}"
                )
            }
        }
        Type::Slice(_) | Type::MutSlice(_) => format!(
            "{{ let values = {rendered}; let index = {index}; axiom_array_get(values, index) }}"
        ),
        _ => unreachable!("type checker rejects first/last on non-collection values"),
    }
}

impl crate::mir::CompareOp {
    fn lexeme(self) -> &'static str {
        match self {
            crate::mir::CompareOp::Eq => "==",
            crate::mir::CompareOp::Ne => "!=",
            crate::mir::CompareOp::Lt => "<",
            crate::mir::CompareOp::Le => "<=",
            crate::mir::CompareOp::Gt => ">",
            crate::mir::CompareOp::Ge => ">=",
        }
    }
}

pub fn compile_native(
    generated_rust: &Path,
    binary_path: &Path,
    target: Option<&str>,
) -> Result<(), Diagnostic> {
    let mut command = Command::new("rustc");
    command
        .arg("--crate-name")
        .arg("axiom_stage1_bootstrap")
        .arg("--edition=2024")
        .arg("-O");
    if let Some(target) = target {
        command.arg("--target").arg(target);
    }
    let status = command
        .arg(generated_rust)
        .arg("-o")
        .arg(binary_path)
        .status()
        .map_err(|err| {
            Diagnostic::new("build", format!("failed to invoke rustc: {err}"))
                .with_path(generated_rust.display().to_string())
        })?;
    if !status.success() {
        return Err(
            Diagnostic::new("build", "rustc failed to produce a native binary")
                .with_path(generated_rust.display().to_string()),
        );
    }
    Ok(())
}
