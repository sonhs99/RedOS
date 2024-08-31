[ORG 0x8000]
[BITS 16]
jmp START16

align 4, db 0
PAGE_TABLE_PTR: dd 0x00
AP_ENTRY_POINT: dq 0x00
STACK_ADDR: dd 0x00 
STACK_SIZE: dd 0x00

START16:
    cli
    lgdt [GDTR]
    
    mov eax, 0x4000003B
    mov cr0, eax

    jmp dword 0x18:START32

[BITS 32]
START32:
    mov ax, 0x20
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    mov eax, cr4
    or eax, 0x620
    mov cr4, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x0101
    wrmsr

    mov eax, [PAGE_TABLE_PTR]
    mov cr3, eax

    mov eax, cr0
    or  eax, 0xE000000E
    xor eax, 0x6000000C
    mov cr0, eax
    
    jmp 0x08:START64

align 8, db 0

[BITS 64]
START64:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    xor rax, rax
    mov eax, [STACK_ADDR]
    mov rsp, rax
    mov rbp, rax


    xor rax, rax
    mov rbx, 0xFEE00020
    mov eax, dword [rbx]
    shr rax, 24

    xor rbx, rbx
    mov ebx, dword [STACK_SIZE]
    mul rbx

    sub rsp, rax
    sub rbp, rax

    push qword ENDLESS
    mov rax, qword [AP_ENTRY_POINT]
    jmp rax

ENDLESS:
    jmp ENDLESS

align 8, db 0
dw 0x0000

GDTR:
    dw GDTEND - GDT - 1
    dd GDT

GDT:
NULL:
    dw 0x0000
    dw 0x0000
    db 0x00
    db 0x00
    db 0x00
    db 0x00

CODE64:
    dw 0xFFFF
    dw 0x0000
    db 0x00
    db 0x9A
    db 0xAF
    db 0x00

DATA64:
    dw 0xFFFF
    dw 0x0000
    db 0x00
    db 0x92
    db 0xAF
    db 0x00

CODE32:
    dw 0xFFFF
    dw 0x0000
    db 0x00
    db 0x9A
    db 0xCF
    db 0x00

DATA32:
    dw 0xFFFF
    dw 0x0000
    db 0x00
    db 0x92
    db 0xCF
    db 0x00
GDTEND:
