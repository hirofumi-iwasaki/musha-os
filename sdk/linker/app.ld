/* Experimental image range; OS loader is a later stage. */
ENTRY(_start)
PHDRS { text PT_LOAD FLAGS(5); ro PT_LOAD FLAGS(4); note PT_NOTE FLAGS(4); data PT_LOAD FLAGS(6); stack PT_GNU_STACK FLAGS(6); }
SECTIONS {
 . = 0x400000;
 .text : { *(.text.start) *(.text .text.*) } :text
 . = ALIGN(4096);
 .rodata : { *(.rodata .rodata.*) } :ro
 .note.musha : { KEEP(*(.note.musha)) } :ro :note
 . = ALIGN(4096);
 .data : { *(.data .data.*) } :data
 .bss : { *(.bss .bss.*) *(COMMON) } :data
 /DISCARD/ : { *(.eh_frame*) *(.comment) }
}
