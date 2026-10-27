# Interrupts & Exceptions

> Status: **implemented** (M1, #10). Hand-rolled per
> [ADR 0009](../adr/0009-hand-rolled-cpu-tables.md). Hardware IRQ delivery
> (APIC) arrives in M3.

## Flow

```
CPU raises vector V
  │  pushes SS, RSP, RFLAGS, CS, RIP  (+ error code for some vectors)
  ▼
vector_stub_V            (generated asm, one per vector)
  │  • push 0 as a dummy error code if the CPU didn't push one
  │  • push V
  │  jmp common_stub
  ▼
common_stub              (one shared asm routine)
  │  • push all 15 GPRs  → the stack now holds a full TrapFrame
  │  • rdi = rsp (&mut TrapFrame); 16-byte-align rsp
  │  • call idt_dispatch
  │  • restore GPRs, drop vector+error_code, iretq
  ▼
idt_dispatch(&mut TrapFrame)   (Rust)
  │  • HANDLERS[vector] set?  → call it
  │  • else                   → default_exception()
  ▼
handler returns → common_stub restores state → iretq resumes
```

## TrapFrame

`hal::idt::TrapFrame` (`#[repr(C)]`) mirrors the stack layout the stubs build:
the 15 general-purpose registers, then `vector` and `error_code`, then the
CPU-pushed frame (`rip`, `cs`, `rflags`, `rsp`, `ss`). Handlers receive
`&mut TrapFrame`, so they can both inspect and modify saved state (needed later
for scheduling and signal-like delivery).

## Error-code normalization

Vectors 8, 10, 11, 12, 13, 14, 17, and 21 push a CPU error code; the rest do
not. Each stub pushes a dummy `0` for the no-error-code vectors so every
`TrapFrame` has the same shape.

## Dispatch & registration

Dispatch is **table-driven**: a 256-entry table of handler function pointers
(`register_handler(vector, fn)`). This is the single hook for CPU exceptions,
hardware IRQs (M3), IPIs, and anything else. Vectors 0–31 are reserved by the
architecture for exceptions; 32–255 are available for IRQs and software use.

A vector with no registered handler falls back to `default_exception`, which
prints a diagnostic report (vector name, error code, `CR2` for page faults, and
a register dump) over serial. `#BP` (breakpoint) is a trap and resumes; any
other unhandled CPU exception is treated as unrecoverable and halts. (A richer
`#PF` handler and `panic!`-based reporting with a backtrace come with #11.)

## IST stacks

The IDT gates for the double fault (#DF → IST1), NMI (→ IST2), and machine
check (#MC → IST3) use the Interrupt Stack Table entries set up in
[`hal::gdt`](memory-layout.md), so these always execute on a known-good stack
even if the kernel stack is corrupt or overflowed.
