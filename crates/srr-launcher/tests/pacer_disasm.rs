//! Decodes the caves and site bytes with iced-x86 and checks, per
//! invariant: everything decodes, every path ends in the right branch back
//! (or `ret`), relocated instructions are byte-identical to the originals,
//! the stack balances and the registers the surrounding game code relies
//! on are preserved. A small abstract interpreter walks every path.

use std::collections::HashMap;

use iced_x86::{
    Code, Decoder, DecoderOptions, FlowControl, Instruction, InstructionInfoFactory, Mnemonic,
    OpAccess, OpKind, Register,
};
use srr_launcher::pacer::*;

const ENTER_CS_IAT: u64 = 0x68_6214;
const LEAVE_CS_IAT: u64 = 0x68_6218;
const MASTER_UPDATE: u64 = 0x58_9890;
const AULLDIV: u64 = 0x4A_F8F0;

fn decode_all(bytes: &[u8], ip: u64) -> Vec<Instruction> {
    let mut d = Decoder::with_ip(32, bytes, ip, DecoderOptions::NONE);
    let mut out = Vec::new();
    while d.can_decode() {
        let i = d.decode();
        assert!(!i.is_invalid(), "invalid instruction at {:#x}", i.ip());
        out.push(i);
    }
    out
}

fn cave() -> HashMap<u64, Instruction> {
    // Only the three routines; the 0xCC padding between them decodes as int3.
    decode_all(&CAVE, u64::from(CAVE_VA))
        .into_iter()
        .map(|i| (i.ip(), i))
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Val {
    /// The value the register had on entry.
    Orig(usize),
    Junk,
}

const GPRS: [Register; 8] = [
    Register::EAX,
    Register::ECX,
    Register::EDX,
    Register::EBX,
    Register::ESP,
    Register::EBP,
    Register::ESI,
    Register::EDI,
];
const EAX: usize = 0;
const ECX: usize = 1;
const EDX: usize = 2;
const EBX: usize = 3;
const ESP: usize = 4;
const EBP: usize = 5;
const ESI: usize = 6;
const EDI: usize = 7;

fn gpr(r: Register) -> Option<usize> {
    let full = r.full_register32();
    GPRS.iter().position(|&g| g == full)
}

#[derive(Clone, Debug)]
struct State {
    regs: [Val; 8],
    stack: Vec<Val>,
    visits: HashMap<u64, u8>,
}

#[derive(Debug, PartialEq)]
enum End {
    Ret,
    Jmp(u64),
}

/// Every path from `entry`: (how it ends, the state there).
fn walk(code: &HashMap<u64, Instruction>, entry: u64) -> Vec<(End, State)> {
    let start = State {
        regs: std::array::from_fn(Val::Orig),
        stack: Vec::new(),
        visits: HashMap::new(),
    };
    let mut done = Vec::new();
    let mut todo = vec![(entry, start, 0usize)];
    let mut info = InstructionInfoFactory::new();
    while let Some((ip, mut st, steps)) = todo.pop() {
        assert!(steps < 200, "path too long at {ip:#x}");
        // A loop (the cmpxchg8b retry) is followed for two iterations;
        // after that the path only repeats states already checked.
        if *st.visits.entry(ip).or_insert(0) >= 2 {
            continue;
        }
        *st.visits.get_mut(&ip).unwrap() += 1;
        let i = code
            .get(&ip)
            .unwrap_or_else(|| panic!("no instruction at {ip:#x}"));
        assert_ne!(
            i.mnemonic(),
            Mnemonic::Int3,
            "path runs into padding at {ip:#x}"
        );
        let next = i.next_ip();
        match i.mnemonic() {
            Mnemonic::Push => {
                let v = if i.op0_kind() == OpKind::Register {
                    st.regs[gpr(i.op0_register()).unwrap()]
                } else {
                    Val::Junk
                };
                st.stack.push(v);
            }
            Mnemonic::Pop => {
                let v = st.stack.pop().expect("pop on empty stack");
                st.regs[gpr(i.op0_register()).unwrap()] = v;
            }
            Mnemonic::Ret => {
                done.push((End::Ret, st));
                continue;
            }
            Mnemonic::Call => {
                let (clobber, pops) = if i.op0_kind() == OpKind::Memory {
                    let slot = i.memory_displacement64();
                    assert!(
                        slot == ENTER_CS_IAT || slot == LEAVE_CS_IAT,
                        "unexpected indirect call {slot:#x}"
                    );
                    (vec![EAX, ECX, EDX], 1) // stdcall, one argument
                } else {
                    match i.near_branch_target() {
                        MASTER_UPDATE => (vec![EAX, ECX, EDX], 0),
                        AULLDIV => (vec![EAX, ECX, EDX], 4), // ret 0x10
                        t if t == u64::from(STORE_RACE_CLOCK) || t == u64::from(PACER_STORE) => {
                            (vec![EAX, EDX], 0)
                        }
                        t => panic!("unexpected call target {t:#x}"),
                    }
                };
                for r in clobber {
                    st.regs[r] = Val::Junk;
                }
                for _ in 0..pops {
                    st.stack.pop().expect("callee pops more than was pushed");
                }
            }
            _ => match i.flow_control() {
                FlowControl::UnconditionalBranch => {
                    let t = i.near_branch_target();
                    if code.contains_key(&t)
                        && t >= u64::from(CAVE_VA)
                        && t < u64::from(CAVE_VA) + CAVE_LEN as u64
                    {
                        todo.push((t, st, steps + 1));
                    } else {
                        done.push((End::Jmp(t), st));
                    }
                    continue;
                }
                FlowControl::ConditionalBranch => {
                    todo.push((i.near_branch_target(), st.clone(), steps + 1));
                }
                FlowControl::Next => {
                    let used = info.info(i);
                    for r in used.used_registers() {
                        let writes = matches!(
                            r.access(),
                            OpAccess::Write
                                | OpAccess::CondWrite
                                | OpAccess::ReadWrite
                                | OpAccess::ReadCondWrite
                        );
                        if writes && let Some(g) = gpr(r.register()) {
                            if g == ESP {
                                // Only the relocated `and esp,-8` touches ESP.
                                assert_eq!(
                                    i.code(),
                                    Code::And_rm32_imm8,
                                    "ESP written at {:#x}",
                                    i.ip()
                                );
                            } else if i.code() == Code::Mov_r32_rm32
                                && i.op1_kind() == OpKind::Register
                            {
                                st.regs[g] = st.regs[gpr(i.op1_register()).unwrap()];
                            } else {
                                st.regs[g] = Val::Junk;
                            }
                        }
                    }
                }
                other => panic!("unexpected flow {other:?} at {:#x}", i.ip()),
            },
        }
        todo.push((next, st, steps + 1));
    }
    done
}

fn kept(st: &State, regs: &[usize]) -> bool {
    regs.iter().all(|&r| st.regs[r] == Val::Orig(r))
}

#[test]
fn store_routines_balance_and_keep_callee_saved_registers() {
    for entry in [STORE_RACE_CLOCK, PACER_STORE] {
        let paths = walk(&cave(), u64::from(entry));
        assert!(paths.len() >= 2, "store and keep paths from {entry:#x}");
        for (end, st) in &paths {
            assert_eq!(*end, End::Ret);
            assert!(st.stack.is_empty(), "stack balanced: {st:?}");
            assert!(kept(st, &[EBX, ECX, ESI, EDI, EBP]), "{st:?}");
        }
    }
}

#[test]
fn pacer_store_defers_to_a_recent_frame_hook() {
    // pacer_store: EBX=1 then the shared body; store_race_clock: EBX=0.
    let ins = decode_all(
        &CAVE[..(SPAWN_GUARD - CAVE_VA) as usize],
        u64::from(CAVE_VA),
    );
    let at = |ip: u64| ins.iter().find(|i| i.ip() == ip).unwrap();
    assert_eq!(
        at(u64::from(STORE_RACE_CLOCK) + 2).mnemonic(),
        Mnemonic::Xor
    );
    let set = at(u64::from(PACER_STORE) + 2);
    assert_eq!((set.op0_register(), set.immediate32()), (Register::EBX, 1));
    // The hold test: (master ms - LAST_HOOK_MS) < HOOK_HOLD_MS, unsigned.
    let sub = ins.iter().find(|i| i.mnemonic() == Mnemonic::Sub).unwrap();
    assert_eq!(sub.memory_displacement64(), u64::from(LAST_HOOK_MS));
    let cmp = ins
        .iter()
        .find(|i| i.mnemonic() == Mnemonic::Cmp && i.op1_kind() == OpKind::Immediate8to32)
        .unwrap();
    assert_eq!(cmp.immediate32(), HOOK_HOLD_MS);
    let skip = ins
        .iter()
        .skip_while(|i| i.ip() != cmp.ip())
        .nth(1)
        .unwrap();
    assert_eq!(
        skip.mnemonic(),
        Mnemonic::Jb,
        "unsigned: wraps every 49 days harmlessly"
    );
    // The frame hook records its store for the pacer.
    let h = (FRAME_HOOK - CAVE_VA) as usize;
    let rec = &decode_all(&CAVE[h + 5..h + 10], u64::from(FRAME_HOOK) + 5)[0];
    assert_eq!(
        (
            rec.mnemonic(),
            rec.memory_displacement64(),
            rec.op1_register()
        ),
        (Mnemonic::Mov, u64::from(LAST_HOOK_MS), Register::EAX)
    );
}

#[test]
fn store_race_clock_takes_the_clock_lock_and_stores_atomically() {
    let ins = decode_all(
        &CAVE[..(SPAWN_GUARD - CAVE_VA) as usize],
        u64::from(CAVE_VA),
    );
    let calls: Vec<String> = ins
        .iter()
        .filter(|i| i.mnemonic() == Mnemonic::Call)
        .map(|i| {
            if i.op0_kind() == OpKind::Memory {
                format!("[{:#x}]", i.memory_displacement64())
            } else {
                format!("{:#x}", i.near_branch_target())
            }
        })
        .collect();
    assert_eq!(
        calls,
        ["[0x686214]", "0x589890", "0x4af8f0", "[0x686218]"],
        "Enter, update, divide, Leave"
    );
    // Both lock calls take 0xA13480.
    let pushes_cs = ins
        .iter()
        .filter(|i| i.code() == Code::Pushd_imm32 && i.immediate32() == 0xA1_3480)
        .count();
    assert_eq!(pushes_cs, 2);
    // Divisor 1000 (µs -> ms), dividend = master µs at 0x950928.
    assert!(
        ins.iter()
            .any(|i| i.code() == Code::Pushd_imm32 && i.immediate32() == 1000)
    );
    assert!(ins.iter().any(|i| i.mnemonic() == Mnemonic::Push
        && i.op0_kind() == OpKind::Memory
        && i.memory_displacement64() == 0x95_0928));
    assert!(ins.iter().any(|i| i.mnemonic() == Mnemonic::Push
        && i.op0_kind() == OpKind::Memory
        && i.memory_displacement64() == 0x95_092C));
    // The only memory write is lock cmpxchg8b on the race clock.
    let mut info = InstructionInfoFactory::new();
    for i in &ins {
        let mem_writes: Vec<_> = info
            .info(i)
            .used_memory()
            .iter()
            .filter(|m| {
                matches!(
                    m.access(),
                    OpAccess::Write
                        | OpAccess::CondWrite
                        | OpAccess::ReadWrite
                        | OpAccess::ReadCondWrite
                )
            })
            .filter(|m| m.base() != Register::ESP)
            .map(|m| m.displacement())
            .collect();
        if !mem_writes.is_empty() {
            assert_eq!(i.mnemonic(), Mnemonic::Cmpxchg8b);
            assert!(i.has_lock_prefix());
            assert_eq!(mem_writes, [0x96_70D8]);
        }
    }
    // Unsigned 64-bit max: jb/ja on the high half, jbe on the low half.
    let jccs: Vec<Mnemonic> = ins
        .iter()
        .filter(|i| i.flow_control() == FlowControl::ConditionalBranch)
        .map(|i| i.mnemonic())
        .collect();
    assert_eq!(
        jccs,
        [
            Mnemonic::Je,
            Mnemonic::Jb,
            Mnemonic::Jb,
            Mnemonic::Ja,
            Mnemonic::Jbe,
            Mnemonic::Jne
        ]
    );
}

#[test]
fn spawn_guard_r1() {
    let code = cave();
    let paths = walk(&code, u64::from(SPAWN_GUARD));
    assert_eq!(paths.len(), 2);
    for (end, st) in &paths {
        assert!(kept(st, &[EBX, ECX, ESI, EDI, EBP]), "{st:?}");
        match end {
            // Handle 0: the three relocated instructions, then back to
            // 0x595660 + 5 with the same stack the original would have.
            End::Jmp(t) => {
                assert_eq!(*t, 0x59_5660 + SPAWN_ORIG.len() as u64);
                assert_eq!(st.stack, [Val::Orig(ECX), Val::Junk]);
            }
            // Handle set: return to the caller, AL = 1, nothing on the stack.
            End::Ret => {
                assert!(st.stack.is_empty());
                assert!(kept(st, &[EDX]));
            }
        }
    }
    // Byte-exact relocation, and the guard reads exactly the handle slot.
    let g = (SPAWN_GUARD - CAVE_VA) as usize;
    assert_eq!(&CAVE[g + 6..g + 11], &SPAWN_ORIG);
    let first = &decode_all(&CAVE[g..g + 4], u64::from(SPAWN_GUARD))[0];
    assert_eq!(
        (
            first.mnemonic(),
            first.memory_base(),
            first.memory_displacement32(),
            first.immediate32()
        ),
        (Mnemonic::Cmp, Register::ESI, 0x1C, 0)
    );
    let alive = decode_all(&CAVE[g + 16..g + 23], u64::from(SPAWN_GUARD) + 16);
    assert_eq!(
        alive[0].memory_displacement32(),
        0x22,
        "started flag kept true"
    );
    assert_eq!(alive[0].immediate8(), 1);
    assert_eq!(
        (alive[1].op0_register(), alive[1].immediate8()),
        (Register::AL, 1)
    );
}

#[test]
fn frame_hook_r3() {
    let code = cave();
    let paths = walk(&code, u64::from(FRAME_HOOK));
    assert_eq!(paths.len(), 1);
    let (end, st) = &paths[0];
    assert_eq!(*end, End::Jmp(0x5D_31F0 + KF_UPDATE_ORIG.len() as u64));
    // `this` (ECX) and the callee-saved registers reach the body intact;
    // EBP is the new frame pointer exactly as the original prologue makes it.
    assert!(kept(st, &[ECX, EBX, ESI, EDI]), "{st:?}");
    assert_eq!(st.stack, [Val::Orig(EBP)]);
    let h = (FRAME_HOOK - CAVE_VA) as usize;
    let first = &decode_all(&CAVE[h..h + 5], u64::from(FRAME_HOOK))[0];
    assert_eq!(
        (first.mnemonic(), first.near_branch_target()),
        (Mnemonic::Call, u64::from(STORE_RACE_CLOCK))
    );
    assert_eq!(
        &CAVE[h + 10..h + 16],
        &KF_UPDATE_ORIG,
        "prologue relocated byte for byte"
    );
}

#[test]
fn frame_end_hook_r3() {
    let paths = walk(&cave(), u64::from(FRAME_END_HOOK));
    assert_eq!(
        paths.len(),
        1,
        "store_race_clock is summarised (EAX/EDX out), then one ret"
    );
    for (end, st) in &paths {
        assert_eq!(*end, End::Ret);
        assert!(st.stack.is_empty());
        // The replaced `call 0x589890` clobbered EAX/ECX/EDX; this keeps more.
        assert!(kept(st, &[EBX, ECX, ESI, EDI, EBP]), "{st:?}");
    }
    let s = site(&FRAME_END_NEW, 0x58_E9D3);
    assert_eq!(
        (s[0].mnemonic(), s[0].near_branch_target()),
        (Mnemonic::Call, u64::from(FRAME_END_HOOK))
    );
    let orig = &decode_all(&FRAME_END_ORIG, 0x58_E9D3)[0];
    assert_eq!(
        (orig.mnemonic(), orig.near_branch_target()),
        (Mnemonic::Call, 0x58_9890),
        "the master update it replaces"
    );
    let e = (FRAME_END_HOOK - CAVE_VA) as usize;
    let ins = decode_all(&CAVE[e..e + 11], u64::from(FRAME_END_HOOK));
    assert_eq!(ins[0].near_branch_target(), u64::from(STORE_RACE_CLOCK));
    assert_eq!(ins[1].memory_displacement64(), u64::from(LAST_HOOK_MS));
    assert_eq!(ins[2].mnemonic(), Mnemonic::Ret);
}

/// A site's reachable instructions. Execution must leave the site either
/// by an unconditional jmp out (hook sites) or by a short jmp to the site
/// end, i.e. the first original instruction after it; every byte after that
/// jmp is int3 (unreachable filler).
fn site(bytes: &[u8], va: u64) -> Vec<Instruction> {
    let end = va + bytes.len() as u64;
    let mut out = Vec::new();
    for i in decode_all(bytes, va) {
        out.push(i);
        if i.flow_control() == FlowControl::UnconditionalBranch {
            let rest = &bytes[(i.next_ip() - va) as usize..];
            assert!(
                rest.iter().all(|&b| b == 0xCC),
                "filler after the jmp at {:#x}",
                i.ip()
            );
            if i.near_branch_target() == end {
                out.pop(); // the skip itself
            } else {
                assert!(
                    i.near_branch_target() >= u64::from(CAVE_VA)
                        && i.near_branch_target() < u64::from(CAVE_VA) + CAVE_LEN as u64,
                    "jmp out of a site goes to the cave"
                );
            }
            return out;
        }
        assert!(i.next_ip() <= end);
    }
    assert_eq!(
        out.last().unwrap().next_ip(),
        end,
        "site at {va:#x} must end on an instruction boundary"
    );
    out
}

#[test]
fn hook_sites_decode_and_branch_to_the_caves() {
    let s = site(&SPAWN_NEW, 0x59_5660);
    assert_eq!(
        (s.len(), s[0].mnemonic(), s[0].near_branch_target()),
        (1, Mnemonic::Jmp, u64::from(SPAWN_GUARD))
    );
    let s = site(&KF_UPDATE_NEW, 0x5D_31F0);
    assert_eq!(
        (s.len(), s[0].mnemonic(), s[0].near_branch_target()),
        (1, Mnemonic::Jmp, u64::from(FRAME_HOOK))
    );
    for (bytes, va, target) in [
        (&PACER_SNAP_NEW[..], 0x59_5381u64, PACER_STORE),
        (&KF_INIT_NEW[..], 0x65_0655, STORE_RACE_CLOCK),
        (&RACE_INIT_NEW[..], 0x5D_21E5, STORE_RACE_CLOCK),
    ] {
        let s = site(bytes, va);
        assert_eq!(s.len(), 1);
        assert_eq!(
            (s[0].mnemonic(), s[0].near_branch_target()),
            (Mnemonic::Call, u64::from(target))
        );
    }
    // Pacer tick: keeps the schedule store, then the race-clock store.
    let s = site(&PACER_TICK_NEW, 0x59_5323);
    assert_eq!(s.len(), 2);
    assert_eq!(
        &PACER_TICK_NEW[..4],
        &PACER_TICK_ORIG[7..11],
        "mov [esp+0x18],ecx kept byte for byte"
    );
    assert_eq!(
        (s[1].mnemonic(), s[1].near_branch_target()),
        (Mnemonic::Call, u64::from(PACER_STORE))
    );
    // Pacer start: store, then the run-flag test that the following jz uses
    // (nothing after the test touches the flags: only NOPs).
    let s = site(&PACER_START_NEW, 0x59_5249);
    assert_eq!(s.len(), 3);
    assert_eq!(
        (s[0].mnemonic(), s[0].near_branch_target()),
        (Mnemonic::Call, u64::from(STORE_RACE_CLOCK))
    );
    assert_eq!(
        &PACER_START_NEW[5..12],
        &PACER_START_ORIG[35..42],
        "mov al,[0xB420A4] / test al,al kept byte for byte"
    );
}

#[test]
fn originals_hold_what_the_sites_replace() {
    // The originals each site replaces, decoded: proves which game writes
    // are rerouted and that no relative branch was relocated by copying.
    let race_writes = |b: &[u8], va: u64| {
        decode_all(b, va)
            .iter()
            .filter(|i| {
                i.op0_kind() == OpKind::Memory
                    && (i.memory_displacement64() == 0x96_70D8
                        || i.memory_displacement64() == 0x96_70DC)
            })
            .count()
    };
    assert_eq!(race_writes(&PACER_START_ORIG, 0x59_5249), 2);
    assert_eq!(race_writes(&PACER_TICK_ORIG, 0x59_5323), 2);
    assert_eq!(race_writes(&PACER_SNAP_ORIG, 0x59_5381), 2);
    assert_eq!(race_writes(&KF_INIT_ORIG, 0x65_0655), 2);
    assert_eq!(race_writes(&RACE_INIT_ORIG, 0x5D_21E5), 2);
    let tick = decode_all(&PACER_TICK_ORIG, 0x59_5323);
    assert_eq!(
        (tick[0].mnemonic(), tick[0].immediate32()),
        (Mnemonic::Add, 0x10),
        "the +16 ms step"
    );
    for b in [&SPAWN_ORIG[..], &KF_UPDATE_ORIG[..]] {
        assert!(
            decode_all(b, 0)
                .iter()
                .all(|i| i.flow_control() == FlowControl::Next),
            "relocated code has no branches"
        );
    }
}
