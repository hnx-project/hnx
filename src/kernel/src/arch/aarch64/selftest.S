/* AArch64 user-mode selftest snippets */
.section .text
.align 2
.global user_trap_mrs_ttbr0
user_trap_mrs_ttbr0:
    mrs x9, ttbr0_el1
    b .

.global user_svc0
user_svc0:
  movz x8, #0
  svc #0
  b .

.global user_svc_getpid
user_svc_getpid:
  movz x8, #3
  svc #0
  b .

.global user_trap_brk
user_trap_brk:
  brk #0
  b .

.global user_svc_write
user_svc_write:
  movz x8, #1
  svc #0
  movz x0, #0
  movz x8, #2
  svc #0
  b .

.global user_svc_exit
user_svc_exit:
  movz x0, #0
  movz x8, #2
  svc #0
  b .

.global user_svc_generic
user_svc_generic:
  svc #0
  b .

.global user_svc_write_imm1
user_svc_write_imm1:
  svc #1
  b .

.global user_store_ok_svc1
user_store_ok_svc1:
  mov x0, x20
  movz w1, #0x4B4F
  movk w1, #0x000A, lsl #16
  str w1, [x0]
  svc #1
  b .
