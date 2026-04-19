ENTRY(_start)

SECTIONS
{
  . = 0x0;

  .text : ALIGN(16) {
    KEEP(*(.text._start))
    *(.text .text.*)
  }

  .rodata : ALIGN(16) {
    *(.rodata .rodata.*)
  }

  .data : ALIGN(16) {
    *(.data .data.*)
  }

  .bss (NOLOAD) : ALIGN(16) {
    *(.bss .bss.*)
    *(COMMON)
  }

  /DISCARD/ : {
    *(.comment)
    *(.note*)
    *(.eh_frame*)
  }
}
