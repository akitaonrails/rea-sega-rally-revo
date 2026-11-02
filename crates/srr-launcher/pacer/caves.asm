; Sega Rally Revo (SEGA Rally_SSE1.exe v4.0.6.0, SHA-256 22655a54...f76e)
; clean-room `pacer` fix: code cave.
;
; Written from SPEC.md (R1-R4) and the game's own code as read with REA.
; Assemble:  nasm -f bin -o caves.bin pacer/caves.asm   (NASM 2.16+; built with NASM 3.02)
; The bytes in src/patches.rs (CAVE) must equal the output; tests/asm.rs
; re-assembles this file when nasm is on PATH and compares.
;
; The cave lives in .text slack: VA 0x685030..0x68510F (the game's last
; code byte is 0x685027; SPEC.md 4.1 verified 0x685028..0x685FFF zero).
; Padding between routines is 0xCC.
;
; One dword of state, LAST_HOOK_MS, lives at 0xB4B960: past the last
; referenced .data variable (0xB4B958, ends at the old VirtualSize end
; 0xB4B95C) in .data's zero-initialised tail page; no 32-bit value anywhere
; in the image points into 0xB4B95C..0xB4B97F (full-file scan). The set
; raises .data VirtualSize to 0x43C964 so the dword is declared.

bits 32
org 0x685030

; --- game addresses (verified on the clean exe) --------------------------
ENTER_CS        equ 0x686214    ; IAT: EnterCriticalSection
LEAVE_CS        equ 0x686218    ; IAT: LeaveCriticalSection
CLOCK_CS        equ 0xA13480    ; CRITICAL_SECTION 0x589890 runs under
MASTER_UPDATE   equ 0x589890    ; master clock update (keeps EBX ESI EDI EBP)
AULLDIV         equ 0x4AF8F0    ; CRT _aulldiv(u64 a, u64 b), callee pops 16,
                                ; keeps EBX ESI, clobbers ECX
MASTER_US       equ 0x950928    ; u64 master microseconds
RACE_CLOCK      equ 0x9670D8    ; u64 race clock, ms (8-byte aligned)
SPAWN_RESUME    equ 0x595665    ; spawn routine after its first 3 instructions
KF_RESUME       equ 0x5D31F6    ; keyframe update after its prologue
LAST_HOOK_MS    equ 0xB4B960    ; u32: race clock (ms) at the last frame hook
HOOK_HOLD_MS    equ 100         ; pacer stands aside this long after a hook

; Fixed entry points; sites.asm jumps/calls to these.
STORE_RACE_CLOCK equ 0x685030
PACER_STORE      equ 0x685038
SPAWN_GUARD      equ 0x6850C0
FRAME_HOOK       equ 0x6850E0
FRAME_END_HOOK   equ 0x685100
CAVE_END         equ 0x685110

; -------------------------------------------------------------------------
; store_race_clock / pacer_store  (R2, R3, R4)
;   race clock := max(race clock, master_us / 1000), sampled now.
;   pacer_store (the pacer's periodic tick and catch-up snap) skips the
;   store while the per-frame hook has stored within HOOK_HOLD_MS: in a
;   race the main thread alone moves the race clock, once per frame, so
;   it advances by exactly the real time between frames (R3) and the
;   pacer's 16-ms grid cannot add jitter. In the front end and during
;   loads (no hook) the pacer keeps the clock moving every 16 ms, with
;   master time (R2), as before.
;   Out: EDX:EAX = the race clock after the call.
;   Keeps EBX ECX ESI EDI EBP; clobbers EFLAGS.
;
; Every write the patched game makes to the race clock goes through here.
; The master clock is updated and read while holding CLOCK_CS (the lock the
; game's own update takes; critical sections are recursive, and nothing
; else is acquired while it is held, so it cannot deadlock). Holding it
; across sample + store serialises all race-clock writers, so a later
; sample is never overwritten by an earlier one (R4), and the u64 master
; value cannot be read torn. The store is one lock cmpxchg8b, so readers
; on other threads never see a half-written u64; the max() keeps the clock
; monotonic even if some writer outside this routine existed.
; -------------------------------------------------------------------------
store_race_clock:
    push ebx
    push ecx
    xor ebx, ebx                    ; EBX = 0: always store
    jmp short body
    times (PACER_STORE - STORE_RACE_CLOCK) - ($ - $$) db 0xCC
pacer_store:
    push ebx
    push ecx
    mov ebx, 1                      ; EBX = 1: unless a frame hook is recent
body:
    push CLOCK_CS
    call [ENTER_CS]
    call MASTER_UPDATE
    push 0
    push 1000
    push dword [MASTER_US + 4]
    push dword [MASTER_US]
    call AULLDIV                    ; EDX:EAX = master ms
    test ebx, ebx
    jz .store
    mov ecx, eax
    sub ecx, [LAST_HOOK_MS]         ; ms since the last frame hook (mod 2^32)
    cmp ecx, HOOK_HOLD_MS
    jb .skip
.store:
    mov ebx, eax
    mov ecx, edx                    ; ECX:EBX = candidate
    mov eax, [RACE_CLOCK]
    mov edx, [RACE_CLOCK + 4]       ; EDX:EAX = current
.again:
    cmp ecx, edx
    jb .leave
    ja .swap
    cmp ebx, eax
    jbe .leave
.swap:
    lock cmpxchg8b [RACE_CLOCK]     ; ZF=1: stored; else EDX:EAX = current
    jnz .again
    mov eax, ebx
    mov edx, ecx
    jmp short .leave
.skip:
    mov eax, [RACE_CLOCK]
    mov edx, [RACE_CLOCK + 4]
.leave:
    push edx
    push eax
    push CLOCK_CS
    call [LEAVE_CS]
    pop eax
    pop edx
    pop ecx
    pop ebx
    ret

    times (SPAWN_GUARD - STORE_RACE_CLOCK) - ($ - $$) db 0xCC
; -------------------------------------------------------------------------
; spawn_guard  (R1)
;   Entered by a jmp from 0x595660 (spawn; object in ESI, 0xB42084).
;   The handle slot [ESI+0x1C] (0xB420A0) is non-zero exactly while a pacer
;   started by this routine has not been joined: every stop path (0x65C300,
;   0x65CB34, 0x597E30) joins through 0x586FF0, which zeroes it. So:
;   handle != 0 -> a pacer is alive: change nothing, report success
;                  (+0x22 "started" kept true, AL = 1, as the original's
;                  success return);
;   handle == 0 -> run the original routine unchanged.
; -------------------------------------------------------------------------
spawn_guard:
    cmp dword [esi + 0x1C], 0
    jne .alive
    push ecx                        ; relocated: 51        push ecx
    lea eax, [esp]                  ; relocated: 8D 04 24  lea eax,[esp]
    push eax                        ; relocated: 50        push eax
    jmp SPAWN_RESUME
.alive:
    mov byte [esi + 0x22], 1
    mov al, 1
    ret

    times (FRAME_HOOK - STORE_RACE_CLOCK) - ($ - $$) db 0xCC
; -------------------------------------------------------------------------
; frame_hook  (R3)
;   Entered by a jmp from 0x5D31F0, the in-race keyframe update (thiscall,
;   ECX = this, kept). Refreshes the race clock on the calling (main)
;   thread immediately before the function's first read of it (0x5D31FF),
;   which feeds 0x594CB0, and records it in LAST_HOOK_MS so the pacer
;   stops moving the clock between frames. Then runs the relocated
;   prologue and resumes.
;   EAX/EDX are scratch at a thiscall entry and are overwritten by the
;   function before use.
; -------------------------------------------------------------------------
frame_hook:
    call store_race_clock
    mov [LAST_HOOK_MS], eax         ; the pacer stands aside from now on
    push ebp                        ; relocated: 55        push ebp
    db 0x8B, 0xEC                   ; relocated: 8B EC     mov ebp,esp (byte-exact)
    and esp, -8                     ; relocated: 83 E4 F8  and esp,-8
    jmp KF_RESUME

    times (FRAME_END_HOOK - STORE_RACE_CLOCK) - ($ - $$) db 0xCC
; -------------------------------------------------------------------------
; frame_end_hook  (R2, R3)
;   Called instead of `call 0x589890` at 0x58E9D3 in the end-of-frame
;   function 0x58E9B0 (after EndScene, before StretchRect/Present). That
;   call is the master-clock update whose result the function then stores
;   as the frame timestamp (0x9E0D08/0x9E0D10) and frame delta (0x9E0D18).
;   store_race_clock performs that same update (0x589890) and sets the race
;   clock from it, so at every presented frame, in every game state, the
;   race clock equals the frame timestamp in ms. EAX/ECX/EDX were scratch
;   across the original call; EBX/ESI (live: ESI = 0) are kept.
; -------------------------------------------------------------------------
frame_end_hook:
    call store_race_clock
    mov [LAST_HOOK_MS], eax         ; the pacer stands aside from now on
    ret

    times (CAVE_END - STORE_RACE_CLOCK) - ($ - $$) db 0xCC
