//! Straight-line Fp3 programs for the backends: a recorded constraint
//! polynomial (zheng's `air::num` graph, compiled) scheduled for few live
//! values and register-allocated, so a device can evaluate it per row with
//! a bounded scratch.
//!
//! Operands are `u32` with a two-bit tag: input `i` (a column value),
//! constant `i` (an Fp3), register `i`. Values are canonical Fp3 limbs
//! `[c0, c1, c2]` over `t³ = t + 1` (nebu's Fp3).

use nebu::{Fp3, Goldilocks};

/// Canonical Fp3 limbs.
pub type Limbs = [u64; 3];

pub const OP_ADD: u8 = 0;
pub const OP_SUB: u8 = 1;
pub const OP_MUL: u8 = 2;
pub const OP_NEG: u8 = 3;
/// `a · c`, `c` the base part of constant `b` (an operand).
pub const OP_MULB: u8 = 4;

const TAG: u32 = 30;
pub const IN: u32 = 0;
pub const CONST: u32 = 1;
pub const REG: u32 = 2;

pub fn operand(tag: u32, i: u32) -> u32 {
    assert!(i < 1 << TAG);
    (tag << TAG) | i
}
pub fn tag(o: u32) -> u32 {
    o >> TAG
}
pub fn index(o: u32) -> u32 {
    o & ((1 << TAG) - 1)
}

/// One instruction: `reg[dst] = a (op) b`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Op {
    pub code: u8,
    pub a: u32,
    pub b: u32,
    pub dst: u32,
}

/// A scheduled, register-allocated program with one output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub ops: Vec<Op>,
    pub consts: Vec<Limbs>,
    pub inputs: u32,
    pub regs: u32,
    pub output: u32,
}

pub fn to_fp3(x: Limbs) -> Fp3 {
    Fp3::new(Goldilocks::new(x[0]), Goldilocks::new(x[1]), Goldilocks::new(x[2]))
}
pub fn from_fp3(x: Fp3) -> Limbs {
    [x.c0.as_u64(), x.c1.as_u64(), x.c2.as_u64()]
}

impl Program {
    /// From slot form: slots `0..inputs` are inputs, then `consts`, then
    /// one slot per entry of `ops` (`(code, a, b)` over slots; for
    /// `OP_MULB`, `b` indexes `consts`). `output` is a slot. Ops are
    /// re-ordered depth-first from the output (operands just before their
    /// use) and registers reused after each value's last use.
    pub fn from_slots(ops: &[(u8, u32, u32)], consts: Vec<Limbs>, inputs: usize, output: u32) -> Self {
        let base = inputs + consts.len();
        let node = |s: u32| (s as usize).checked_sub(base);
        // depth-first post-order from the output, iteratively
        let mut order = Vec::new();
        let mut seen = vec![false; ops.len()];
        if let Some(o) = node(output) {
            let mut stack: Vec<(usize, bool)> = vec![(o, false)];
            while let Some((i, expanded)) = stack.pop() {
                if expanded {
                    order.push(i);
                    continue;
                }
                if seen[i] {
                    continue;
                }
                seen[i] = true;
                stack.push((i, true));
                let (code, a, b) = ops[i];
                let mut kids = vec![a];
                if code != OP_NEG && code != OP_MULB {
                    kids.push(b);
                }
                // push b first so a is evaluated first
                for &k in kids.iter().rev() {
                    if let Some(j) = node(k) {
                        if !seen[j] {
                            stack.push((j, false));
                        }
                    }
                }
            }
        }
        // last use of every node, in the new order
        let mut last = vec![0usize; ops.len()];
        for (k, &i) in order.iter().enumerate() {
            let (code, a, b) = ops[i];
            for (s, used) in [(a, true), (b, code != OP_NEG && code != OP_MULB)] {
                if used {
                    if let Some(j) = node(s) {
                        last[j] = last[j].max(k);
                    }
                }
            }
        }
        if let Some(o) = node(output) {
            last[o] = usize::MAX;
        }
        let mut reg = vec![u32::MAX; ops.len()];
        let mut free: Vec<u32> = Vec::new();
        let mut regs = 0u32;
        let mut out_ops = Vec::with_capacity(order.len());
        let mut dying: Vec<Vec<u32>> = vec![Vec::new(); order.len()];
        let opnd = |s: u32, reg: &[u32]| -> u32 {
            match node(s) {
                None if (s as usize) < inputs => operand(IN, s),
                None => operand(CONST, s - inputs as u32),
                Some(j) => operand(REG, reg[j]),
            }
        };
        for (k, &i) in order.iter().enumerate() {
            let (code, a, b) = ops[i];
            let oa = opnd(a, &reg);
            let ob = if code == OP_MULB {
                operand(CONST, b)
            } else if code == OP_NEG {
                oa
            } else {
                opnd(b, &reg)
            };
            // operands whose last use is this op free their registers
            // before the result is placed (a result may reuse one)
            for &r in &dying[k] {
                free.push(r);
            }
            let dst = free.pop().unwrap_or_else(|| {
                regs += 1;
                regs - 1
            });
            reg[i] = dst;
            out_ops.push(Op { code, a: oa, b: ob, dst });
            if last[i] != usize::MAX && last[i] > k && last[i] < order.len() {
                dying[last[i]].push(dst);
            } else if last[i] <= k {
                // never used again (cannot happen for a needed node)
                free.push(dst);
            }
        }
        let output = match node(output) {
            None if (output as usize) < inputs => operand(IN, output),
            None => operand(CONST, output - inputs as u32),
            Some(j) => operand(REG, reg[j]),
        };
        Self { ops: out_ops, consts, inputs: inputs as u32, regs, output }
    }

    fn read(&self, o: u32, inputs: &[Fp3], regs: &[Fp3]) -> Fp3 {
        match tag(o) {
            IN => inputs[index(o) as usize],
            CONST => to_fp3(self.consts[index(o) as usize]),
            _ => regs[index(o) as usize],
        }
    }

    /// Evaluate at `inputs` (the reference every backend matches).
    pub fn eval(&self, inputs: &[Fp3], regs: &mut Vec<Fp3>) -> Fp3 {
        regs.clear();
        regs.resize(self.regs as usize, Fp3::ZERO);
        for op in &self.ops {
            let x = self.read(op.a, inputs, regs);
            let r = match op.code {
                OP_ADD => x + self.read(op.b, inputs, regs),
                OP_SUB => x - self.read(op.b, inputs, regs),
                OP_MUL => x * self.read(op.b, inputs, regs),
                OP_NEG => -x,
                _ => {
                    let c = Goldilocks::new(self.consts[index(op.b) as usize][0]);
                    Fp3::new(x.c0 * c, x.c1 * c, x.c2 * c)
                }
            };
            regs[op.dst as usize] = r;
        }
        self.read(self.output, inputs, regs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(k: u64) -> Fp3 {
        Fp3::new(Goldilocks::new(k * 7 + 1), Goldilocks::new(k ^ 3), Goldilocks::new(k * k))
    }

    /// Slot evaluation, unscheduled (the definition).
    fn eval_slots(ops: &[(u8, u32, u32)], consts: &[Limbs], inputs: &[Fp3], output: u32) -> Fp3 {
        let mut v: Vec<Fp3> = inputs.to_vec();
        v.extend(consts.iter().map(|&c| to_fp3(c)));
        for &(code, a, b) in ops {
            let x = v[a as usize];
            v.push(match code {
                OP_ADD => x + v[b as usize],
                OP_SUB => x - v[b as usize],
                OP_MUL => x * v[b as usize],
                OP_NEG => -x,
                _ => {
                    let c = Goldilocks::new(consts[b as usize][0]);
                    Fp3::new(x.c0 * c, x.c1 * c, x.c2 * c)
                }
            });
        }
        v[output as usize]
    }

    #[test]
    fn scheduled_programs_evaluate_as_the_slots() {
        let mut r = 17u64;
        let mut next = || {
            r ^= r << 13;
            r ^= r >> 7;
            r ^= r << 17;
            r
        };
        for trial in 0..40 {
            let inputs = 5 + (trial % 7);
            let consts: Vec<Limbs> = (0..4).map(|k| from_fp3(e(k + trial as u64))).chain([[5, 0, 0]]).collect();
            let base = (inputs + consts.len()) as u32;
            let mut ops = Vec::new();
            for k in 0..200u32 {
                let avail = base + k;
                let code = (next() % 5) as u8;
                let a = (next() % avail as u64) as u32;
                let b = if code == OP_MULB { (consts.len() - 1) as u32 } else { (next() % avail as u64) as u32 };
                ops.push((code, a, b));
            }
            let output = base + 199;
            let p = Program::from_slots(&ops, consts.clone(), inputs, output);
            let ins: Vec<Fp3> = (0..inputs as u64).map(|k| e(k + 100 * trial as u64)).collect();
            let mut regs = Vec::new();
            assert_eq!(p.eval(&ins, &mut regs), eval_slots(&ops, &consts, &ins, output));
            assert!(p.regs as usize <= p.ops.len());
        }
    }
}
