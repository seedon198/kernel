/* Higher-half kernel linker script for x86-64 (see docs/architecture/memory-layout.md
 * and docs/adr/0003-bootloader.md). Modeled on the Limine protocol requirements.
 *
 * Each output section gets its own PT_LOAD segment so the kernel can later map
 * them with exact W^X permissions: .text is R-X, .rodata is R--, .data/.bss are
 * RW-. Section boundary symbols are exported for the memory manager (M2). */

OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

PHDRS
{
    text    PT_LOAD    FLAGS((1 << 2) | (1 << 0)) ; /* R + X */
    rodata  PT_LOAD    FLAGS((1 << 2)) ;            /* R     */
    data    PT_LOAD    FLAGS((1 << 2) | (1 << 1)) ; /* R + W */
}

SECTIONS
{
    /* Topmost 2 GiB of the address space, as the Limine spec mandates. The
     * canonical choice is the start of that region. KASLR (M2) will randomize
     * this base. */
    . = 0xffffffff80000000;
    __kernel_start = .;

    .text : {
        __text_start = .;
        *(.text .text.*)
    } :text
    . = ALIGN(CONSTANT(MAXPAGESIZE));
    __text_end = .;

    .rodata : {
        __rodata_start = .;
        *(.rodata .rodata.*)
    } :rodata
    . = ALIGN(CONSTANT(MAXPAGESIZE));
    __rodata_end = .;

    .data : {
        __data_start = .;
        *(.data .data.*)

        /* Limine scans this range for request magic; the markers bound it. */
        KEEP(*(.requests_start_marker))
        KEEP(*(.requests))
        KEEP(*(.requests_end_marker))
    } :data

    /* .bss must be last in :data so the on-disk image isn't padded with zeros. */
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :data
    . = ALIGN(CONSTANT(MAXPAGESIZE));
    __data_end = .;

    __kernel_end = .;

    /* These can trip up some linkers/hosts and we don't use them. */
    /DISCARD/ : {
        *(.eh_frame*)
        *(.note .note.*)
    }
}
