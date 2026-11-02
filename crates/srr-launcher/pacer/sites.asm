; Sega Rally Revo clean-room `pacer` fix: the in-place site bytes.
; Assemble one site at a time:
;   nasm -f bin -DSITE=<n> -o site<n>.bin pacer/sites.asm
; Each site keeps its original length. The spare bytes after the new code
; are a short jmp to the site end followed by int3 (0xCC) filler, which is
; never executed (no game branch targets the inside of any site; checked
; on the REA listings). A plain NOP run made Ghidra stop disassembling
; partway through it. Original instructions are listed above each site
; (from REA on the clean exe).

bits 32

STORE_RACE_CLOCK equ 0x685030      ; see caves.asm
PACER_STORE      equ 0x685038
SPAWN_GUARD      equ 0x6850C0
FRAME_HOOK       equ 0x6850E0
FRAME_END_HOOK   equ 0x685100
RUN_FLAG         equ 0xB420A4

; Jump over the rest of a site of %1 bytes; int3 the gap.
%macro skip_rest 1
    jmp short %%end
    times %1 - ($ - $$) db 0xCC
%%end:
%endmacro

; After an unconditional jmp: int3 the rest of a site of %1 bytes.
%macro pad_int3 1
    times %1 - ($ - $$) db 0xCC
%endmacro

%if SITE == 1
; 0x595660 spawn entry (R1). Original, 5 bytes:
;   51        push ecx
;   8D 04 24  lea eax,[esp]
;   50        push eax
org 0x595660
    jmp SPAWN_GUARD

%elif SITE == 2
; 0x595249 pacer thread, initial race-clock write (R2, R4). Original, 48 bytes:
;   call 0x589890 / mov ecx,[0x95092C] / mov edx,[0x950928] / push edi(0)
;   push 1000 / push ecx / push edx / call 0x4AF8F0
;   mov [0x9670D8],eax / mov al,[0xB420A4] / test al,al / mov [0x9670DC],edx
; followed by jz 0x5953CD (kept), which consumes test al,al's ZF.
org 0x595249
    call STORE_RACE_CLOCK
    mov al, [RUN_FLAG]
    test al, al
    skip_rest 48

%elif SITE == 3
; 0x595323 pacer tick: "race clock += 16" becomes "race clock := master ms",
; unless the per-frame hook is driving it
; (R2). Original, 18 bytes:
;   add dword [0x9670D8],0x10 / mov [esp+0x18],ecx / adc dword [0x9670DC],0
; EAX/EDX are dead here (next use writes them); ECX/EBX/EDI/EBP are live and
; kept by store_race_clock; flags are recomputed by cmp edi,ebx at 0x595335.
org 0x595323
    mov [esp + 0x18], ecx
    call PACER_STORE
    skip_rest 18

%elif SITE == 4
; 0x595381 pacer catch-up snap (R2, R4), via pacer_store like the tick. Original, 40 bytes:
;   call 0x589890 / mov eax,[0x95092C] / mov ecx,[0x950928] / push edi(0)
;   push 1000 / push eax / push ecx / call 0x4AF8F0
;   mov [0x9670D8],eax / mov [0x9670DC],edx
org 0x595381
    call PACER_STORE
    skip_rest 40

%elif SITE == 5
; 0x650655 keyframe-clock init 0x650630 (R4). Original, 41 bytes:
;   call 0x589890 / mov edx,[0x95092C] / mov eax,[0x950928] / push 0
;   push 1000 / push edx / push eax / call 0x4AF8F0
;   mov [0x9670D8],eax / mov [0x9670DC],edx
; The following code stores EAX/EDX (the race clock) into the object.
org 0x650655
    call STORE_RACE_CLOCK
    skip_rest 41

%elif SITE == 6
; 0x5D21E5 race-state init 0x5D21C0 (R4). Same 41-byte sequence as site 5.
org 0x5D21E5
    call STORE_RACE_CLOCK
    skip_rest 41

%elif SITE == 7
; 0x5D31F0 in-race keyframe update entry (R3). Original, 6 bytes:
;   55 push ebp / 8B EC mov ebp,esp / 83 E4 F8 and esp,-8
org 0x5D31F0
    jmp FRAME_HOOK
    pad_int3 6

%elif SITE == 8
; 0x58E9D3 end-of-frame 0x58E9B0, the master-clock update that becomes the
; frame timestamp (R2, R3). Original, 5 bytes: E8 B8 AE FF FF call 0x589890
org 0x58E9D3
    call FRAME_END_HOOK

%else
%error "SITE must be 1..8"
%endif
