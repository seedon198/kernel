# Memory Layout

> Status: **design**. Finalized in milestone M2.

x86-64 with 4-level paging gives a 48-bit virtual address space in two
canonical halves: user space in the lower half, the kernel in the upper half.
Because this is a **microkernel**, most "system" code (drivers, net, fs) runs
in ordinary **user** address spaces, not in the kernel half.

## Virtual address map (kernel view)

| Start | End | Size | Contents |
|---|---|---|---|
| `0x0000000000000000` | `0x0000000000000fff` | 4 KiB | Unmapped — catches NULL dereferences |
| `0x0000000000001000` | `0x00007fffffffffff` | ~128 TiB | **User space** (per task: a server or tool) |
| `0x0000800000000000` | `0xffff7fffffffffff` | — | Non-canonical hole (#GP on access) |
| `0xffff800000000000` | HHDM + RAM | ≤ 64 TiB | **HHDM** — direct map of physical memory (offset from Limine) |
| `0xffffc00000000000` | `0xffffdfffffffffff` | 32 TiB | Kernel heap / dynamic mappings *(tentative)* |
| *KASLR base* | … | 2 GiB window | **Kernel image** — randomized within the top 2 GiB |

The kernel image sits in the top 2 GiB so code uses `-mcmodel=kernel`-style
sign-extended 32-bit addressing. **KASLR** randomizes its base at boot, so the
table above shows the window, not a fixed address.

## Kernel image sections and W^X

| Section | Permissions |
|---|---|
| `.text` | R-X |
| `.rodata` | R-- (NX) |
| `.data`, `.bss` | RW- (NX) |

Sections are page-aligned in the linker script and mapped with exactly these
permissions once the kernel installs its own page tables. **No page is ever
both writable and executable** (W^X), enforced for kernel and user mappings
alike.

## Isolation

Each user task (server or tool) has its **own page table root** (an
`AddressSpace` capability). Tasks share nothing by default; shared memory
exists only where one task explicitly maps a `Frame` capability it received
over IPC. This is the memory-level expression of the
[capability security model](security-model.md).
